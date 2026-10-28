use std::sync::atomic::{AtomicBool, Ordering};

use text_components::TextComponent;
use vecta_protocol::packet_traits::{CompressionInfo, EncodedPacket};

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
