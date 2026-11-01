use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use glam::IVec3;
use text_components::TextComponent;
use vecta_protocol::packet_traits::{CompressionInfo, EncodedPacket};
use vecta_registry::item_stack::ItemStack;
use vecta_registry::packets::play::{
    C_BLOCK_EVENT, C_CONTAINER_CLOSE, C_CONTAINER_SET_CONTENT, C_CONTAINER_SET_DATA,
    C_CONTAINER_SET_SLOT, C_LEVEL_EVENT, C_OPEN_SCREEN, C_SET_CURSOR_ITEM, C_SOUND,
};
use vecta_registry::sound_event::SoundEventHolder;
use vecta_utils::codec::VarInt;
use vecta_utils::locks::SyncMutex;
use vecta_utils::serial::ReadFrom as _;
use vecta_utils::{BlockPos, Identifier};

use crate::player::PlayerConnection;
use crate::player::connection::NetworkConnection;

#[derive(Default)]
pub(crate) struct TestConnection {
    closed: AtomicBool,
}

impl NetworkConnection for TestConnection {
    fn compression(&self) -> Option<CompressionInfo> {
        None
    }

    fn send_encoded(&self, _packet: EncodedPacket) {}

    fn send_encoded_bundle(&self, _packets: Vec<EncodedPacket>) {}

    fn disconnect_with_reason(&self, _reason: TextComponent) {
        self.close();
    }

    fn tick(&self) {}

    fn latency(&self) -> i32 {
        0
    }

    fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }

    fn closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }
}

/// Every encoded packet the server sent, in send order.
pub(crate) type SentPackets = Arc<SyncMutex<Vec<EncodedPacket>>>;

/// A connection that keeps every encoded packet, so a test can assert what
/// actually reached the client instead of what the server meant to send.
pub(crate) struct RecordingConnection {
    packets: SentPackets,
}

impl NetworkConnection for RecordingConnection {
    fn compression(&self) -> Option<CompressionInfo> {
        None
    }

    fn send_encoded(&self, packet: EncodedPacket) {
        self.packets.lock().push(packet);
    }

    fn send_encoded_bundle(&self, packets: Vec<EncodedPacket>) {
        self.packets.lock().extend(packets);
    }

    fn disconnect_with_reason(&self, _reason: TextComponent) {}

    fn tick(&self) {}

    fn latency(&self) -> i32 {
        0
    }

    fn close(&self) {}

    fn closed(&self) -> bool {
        false
    }
}

/// Builds a connection that records packets, plus the shared packet list.
pub(crate) fn recording_connection() -> (Arc<PlayerConnection>, SentPackets) {
    let packets = Arc::new(SyncMutex::new(Vec::new()));
    let connection = Arc::new(PlayerConnection::Other(Box::new(RecordingConnection {
        packets: Arc::clone(&packets),
    })));
    (connection, packets)
}

/// A decoded `C_BLOCK_EVENT` packet: `(pos, action id, action param, block id)`.
pub(crate) type BroadcastBlockEvent = (BlockPos, u8, u8, i32);

/// Decodes the `C_BLOCK_EVENT` packets in `packets`, dropping everything else.
pub(crate) fn block_events(packets: &SyncMutex<Vec<EncodedPacket>>) -> Vec<BroadcastBlockEvent> {
    packets
        .lock()
        .iter()
        .filter_map(|packet| {
            let mut cursor = Cursor::new(packet.encoded_data.as_slice());
            // Frame length, then the packet id.
            VarInt::read(&mut cursor).ok()?;
            if VarInt::read(&mut cursor).ok()?.0 != C_BLOCK_EVENT {
                return None;
            }
            let pos = BlockPos::read(&mut cursor).ok()?;
            let action_id = u8::read(&mut cursor).ok()?;
            let action_param = u8::read(&mut cursor).ok()?;
            let block_id = VarInt::read(&mut cursor).ok()?.0;
            Some((pos, action_id, action_param, block_id))
        })
        .collect()
}

/// Decodes the `C_CONTAINER_CLOSE` packets in `packets`, dropping everything
/// else, and returns the closed container ids in send order.
pub(crate) fn closed_screens(packets: &SyncMutex<Vec<EncodedPacket>>) -> Vec<i32> {
    packets
        .lock()
        .iter()
        .filter_map(|packet| {
            let mut cursor = Cursor::new(packet.encoded_data.as_slice());
            // Frame length, then the packet id.
            VarInt::read(&mut cursor).ok()?;
            if VarInt::read(&mut cursor).ok()?.0 != C_CONTAINER_CLOSE {
                return None;
            }
            Some(VarInt::read(&mut cursor).ok()?.0)
        })
        .collect()
}

/// A decoded `C_LEVEL_EVENT` packet: `(event type, pos, data)`.
pub(crate) type BroadcastLevelEvent = (i32, BlockPos, i32);

/// Decodes the `C_LEVEL_EVENT` packets in `packets`, dropping everything else.
pub(crate) fn level_events(packets: &SyncMutex<Vec<EncodedPacket>>) -> Vec<BroadcastLevelEvent> {
    packets
        .lock()
        .iter()
        .filter_map(|packet| {
            let mut cursor = Cursor::new(packet.encoded_data.as_slice());
            // Frame length, then the packet id.
            VarInt::read(&mut cursor).ok()?;
            if VarInt::read(&mut cursor).ok()?.0 != C_LEVEL_EVENT {
                return None;
            }
            let event_type = i32::read(&mut cursor).ok()?;
            let pos = BlockPos::read(&mut cursor).ok()?;
            let data = i32::read(&mut cursor).ok()?;
            Some((event_type, pos, data))
        })
        .collect()
}

/// A decoded `C_SOUND` packet: `(sound, source, position at 8x, volume)`.
pub(crate) type PlayedSound = (Identifier, i32, IVec3, f32);
/// Decodes the `C_SOUND` packets in `packets`, dropping everything else.
pub(crate) fn played_sounds(packets: &SyncMutex<Vec<EncodedPacket>>) -> Vec<PlayedSound> {
    packets
        .lock()
        .iter()
        .filter_map(|packet| {
            let mut cursor = Cursor::new(packet.encoded_data.as_slice());
            // Frame length, then the packet id.
            VarInt::read(&mut cursor).ok()?;
            if VarInt::read(&mut cursor).ok()?.0 != C_SOUND {
                return None;
            }
            let SoundEventHolder::Registry(sound) = SoundEventHolder::read(&mut cursor).ok()?
            else {
                return None;
            };
            let source = VarInt::read(&mut cursor).ok()?.0;
            let x = i32::read(&mut cursor).ok()?;
            let y = i32::read(&mut cursor).ok()?;
            let z = i32::read(&mut cursor).ok()?;
            let volume = f32::read(&mut cursor).ok()?;
            Some((sound.key.clone(), source, IVec3::new(x, y, z), volume))
        })
        .collect()
}

/// A decoded `C_OPEN_SCREEN` packet: `(container id, menu type id)`.
pub(crate) type OpenedScreen = (i32, i32);

/// Decodes the `C_OPEN_SCREEN` packets in `packets`, dropping everything else.
pub(crate) fn opened_screens(packets: &SyncMutex<Vec<EncodedPacket>>) -> Vec<OpenedScreen> {
    packets
        .lock()
        .iter()
        .filter_map(|packet| {
            let mut cursor = Cursor::new(packet.encoded_data.as_slice());
            // Frame length, then the packet id.
            VarInt::read(&mut cursor).ok()?;
            if VarInt::read(&mut cursor).ok()?.0 != C_OPEN_SCREEN {
                return None;
            }
            let container_id = VarInt::read(&mut cursor).ok()?.0;
            let menu_type = VarInt::read(&mut cursor).ok()?.0;
            Some((container_id, menu_type))
        })
        .collect()
}

/// A decoded `C_CONTAINER_SET_CONTENT` packet: `(container id, state id, every
/// slot in menu order, carried item)`.
pub(crate) type ContainerContents = (i32, i32, Vec<ItemStack>, ItemStack);

/// Decodes the `C_CONTAINER_SET_CONTENT` packets in `packets`, dropping
/// everything else.
pub(crate) fn container_contents(
    packets: &SyncMutex<Vec<EncodedPacket>>,
) -> Vec<ContainerContents> {
    packets
        .lock()
        .iter()
        .filter_map(|packet| {
            let mut cursor = Cursor::new(packet.encoded_data.as_slice());
            // Frame length, then the packet id.
            VarInt::read(&mut cursor).ok()?;
            if VarInt::read(&mut cursor).ok()?.0 != C_CONTAINER_SET_CONTENT {
                return None;
            }
            let container_id = VarInt::read(&mut cursor).ok()?.0;
            let state_id = VarInt::read(&mut cursor).ok()?.0;
            let count = VarInt::read(&mut cursor).ok()?.0;
            let mut items = Vec::with_capacity(count.max(0) as usize);
            for _ in 0..count {
                items.push(ItemStack::read(&mut cursor).ok()?);
            }
            let carried = ItemStack::read(&mut cursor).ok()?;
            Some((container_id, state_id, items, carried))
        })
        .collect()
}

/// A decoded `C_CONTAINER_SET_SLOT` packet: `(container id, state id, slot,
/// item)`.
pub(crate) type ContainerSlotChange = (i32, i32, i16, ItemStack);

/// Decodes the `C_CONTAINER_SET_SLOT` packets in `packets`, dropping everything
/// else.
pub(crate) fn container_slot_changes(
    packets: &SyncMutex<Vec<EncodedPacket>>,
) -> Vec<ContainerSlotChange> {
    packets
        .lock()
        .iter()
        .filter_map(|packet| {
            let mut cursor = Cursor::new(packet.encoded_data.as_slice());
            // Frame length, then the packet id.
            VarInt::read(&mut cursor).ok()?;
            if VarInt::read(&mut cursor).ok()?.0 != C_CONTAINER_SET_SLOT {
                return None;
            }
            let container_id = VarInt::read(&mut cursor).ok()?.0;
            let state_id = VarInt::read(&mut cursor).ok()?.0;
            let slot = i16::read(&mut cursor).ok()?;
            let item = ItemStack::read(&mut cursor).ok()?;
            Some((container_id, state_id, slot, item))
        })
        .collect()
}

/// A decoded `C_CONTAINER_SET_DATA` packet: `(container id, property id,
/// value)`.
pub(crate) type ContainerDataChange = (i32, i16, i16);

/// Decodes the `C_CONTAINER_SET_DATA` packets in `packets`, dropping everything
/// else.
pub(crate) fn container_data_changes(
    packets: &SyncMutex<Vec<EncodedPacket>>,
) -> Vec<ContainerDataChange> {
    packets
        .lock()
        .iter()
        .filter_map(|packet| {
            let mut cursor = Cursor::new(packet.encoded_data.as_slice());
            // Frame length, then the packet id.
            VarInt::read(&mut cursor).ok()?;
            if VarInt::read(&mut cursor).ok()?.0 != C_CONTAINER_SET_DATA {
                return None;
            }
            let container_id = VarInt::read(&mut cursor).ok()?.0;
            let id = i16::read(&mut cursor).ok()?;
            let value = i16::read(&mut cursor).ok()?;
            Some((container_id, id, value))
        })
        .collect()
}

/// Decodes the `C_SET_CURSOR_ITEM` packets in `packets`, dropping everything
/// else.
pub(crate) fn cursor_items(packets: &SyncMutex<Vec<EncodedPacket>>) -> Vec<ItemStack> {
    packets
        .lock()
        .iter()
        .filter_map(|packet| {
            let mut cursor = Cursor::new(packet.encoded_data.as_slice());
            // Frame length, then the packet id.
            VarInt::read(&mut cursor).ok()?;
            if VarInt::read(&mut cursor).ok()?.0 != C_SET_CURSOR_ITEM {
                return None;
            }
            ItemStack::read(&mut cursor).ok()
        })
        .collect()
}
