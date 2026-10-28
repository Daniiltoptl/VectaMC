use std::io::Cursor;
use std::sync::Arc;

use glam::{DVec3, IVec3};
use text_components::TextComponent;
use vecta_protocol::packet_traits::{CompressionInfo, EncodedPacket};
use vecta_registry::RegistryHolder;
use vecta_registry::data_components::components::InstrumentComponent;
use vecta_registry::data_components::vanilla_components::{DAMAGE, INSTRUMENT, MAX_DAMAGE};
use vecta_registry::game_events::GameEventRef;
use vecta_registry::instrument::InstrumentValue;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::packets::play::{C_COOLDOWN, C_SOUND};
use vecta_registry::sound_event::SoundEventHolder;
use vecta_registry::stat::{Stat, vanilla_stat_types};
use vecta_registry::{vanilla_game_events, vanilla_items};
use vecta_utils::codec::VarInt;
use vecta_utils::locks::SyncMutex;
use vecta_utils::serial::ReadFrom as _;
use vecta_utils::types::{GameType, InteractionHand};
use vecta_utils::{BlockPos, ChunkPos, Identifier, SectionPos};

use crate::behavior::InteractionResult;
use crate::bootstrap::init_globals;
use crate::entity::{Entity, LivingEntity as _, next_entity_id};
use crate::player::connection::NetworkConnection;
use crate::player::game_mode::use_item;
use crate::player::{Player, PlayerConnection, ResetReason};
use crate::test_support::{TestPlayerBuilder, fresh_test_world, insert_ready_full_chunk};
use crate::world::World;
use crate::world::game_event::{GameEventContext, GameEventListener};

struct InstrumentListener(SyncMutex<Vec<(GameEventRef, Option<i32>, DVec3)>>);

impl GameEventListener for InstrumentListener {
    fn listener_pos(&self) -> Option<DVec3> {
        Some(DVec3::new(0.5, 80.0, 0.5))
    }
    fn listener_radius(&self) -> i32 {
        16
    }
    fn handle_game_event(
        &self,
        _world: &Arc<World>,
        event: GameEventRef,
        context: &GameEventContext<'_>,
        pos: DVec3,
    ) -> bool {
        self.0
            .lock()
            .push((event, context.source_entity().map(Entity::id), pos));
        true
    }
}

#[derive(Default)]
struct RecordingConnection(SyncMutex<Vec<EncodedPacket>>);

impl NetworkConnection for Arc<RecordingConnection> {
    fn compression(&self) -> Option<CompressionInfo> {
        None
    }
    fn send_encoded(&self, packet: EncodedPacket) {
        self.0.lock().push(packet);
    }
    fn send_encoded_bundle(&self, packets: Vec<EncodedPacket>) {
        self.0.lock().extend(packets);
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

fn player(world: &Arc<World>, x: f64) -> (Arc<Player>, Arc<RecordingConnection>) {
    let connection = Arc::new(RecordingConnection::default());
    let id = next_entity_id();
    let player = TestPlayerBuilder::new(Arc::clone(world), format!("HornTester{id}"), id)
        .connection(Arc::new(PlayerConnection::Other(Box::new(Arc::clone(
            &connection,
        )))))
        .build();
    player
        .try_set_position(DVec3::new(x, 80.0, 0.5))
        .expect("position horn tester");
    (player, connection)
}

fn custom_horn(duration: f32, damage: i32) -> ItemStack {
    let mut horn = ItemStack::new(&vanilla_items::GOAT_HORN);
    horn.set(
        INSTRUMENT,
        InstrumentComponent::new(RegistryHolder::direct(
            InstrumentValue::new(
                SoundEventHolder::Direct {
                    sound_id: Identifier::vanilla_static("test_horn"),
                    fixed_range: Some(16.0),
                },
                duration,
                48.0,
                damage,
                TextComponent::plain("Test horn"),
            )
            .expect("valid custom instrument"),
        )),
    );
    horn
}

fn payloads(connection: &RecordingConnection, id: i32) -> Vec<Vec<u8>> {
    connection
        .0
        .lock()
        .iter()
        .filter_map(|packet| {
            let bytes = packet.encoded_data.as_slice();
            let mut cursor = Cursor::new(bytes);
            VarInt::read(&mut cursor).ok()?;
            if VarInt::read(&mut cursor).ok()?.0 != id {
                return None;
            }
            Some(bytes[cursor.position() as usize..].to_vec())
        })
        .collect()
}

#[test]
fn horn_uses_either_hand_and_blocks_repeated_use_without_consuming_stack() {
    init_globals();
    let fixture = fresh_test_world("instrument_hands");
    for hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
        let (player, connection) = player(&fixture.world, 0.5);
        player
            .inventory
            .lock()
            .set_item_in_hand(hand, ItemStack::new(&vanilla_items::GOAT_HORN));
        assert_eq!(
            use_item(&player, &fixture.world, hand),
            InteractionResult::Consume
        );
        let active = player
            .living_base()
            .active_item_use()
            .expect("horn is being used");
        assert_eq!(active.hand(), hand);
        assert_eq!(active.remaining_ticks(), 140);
        assert_eq!(
            use_item(&player, &fixture.world, hand),
            InteractionResult::Pass
        );
        let inventory = player.inventory.lock();
        let stack = inventory.get_item_in_hand(hand);
        assert_eq!(stack.count(), 1);
        assert!(player.is_item_on_cooldown(stack));
        assert!(player.stats().contains(&(
            Stat::new(&vanilla_stat_types::ITEM_USED, &vanilla_items::GOAT_HORN),
            1
        )));
        let packets = payloads(&connection, C_COOLDOWN);
        assert_eq!(packets.len(), 1);
        let mut cursor = Cursor::new(packets[0].as_slice());
        assert_eq!(
            Identifier::read(&mut cursor).expect("cooldown group"),
            vanilla_items::GOAT_HORN.key
        );
        assert_eq!(VarInt::read(&mut cursor).expect("cooldown duration").0, 140);
    }
}

#[test]
fn missing_instrument_fails_and_zero_duration_still_starts_using() {
    init_globals();
    let fixture = fresh_test_world("instrument_optional");
    let (player, connection) = player(&fixture.world, 0.5);
    let hand = InteractionHand::MainHand;
    let mut horn = custom_horn(0.0, 0);
    horn.remove(INSTRUMENT);
    player.inventory.lock().set_item_in_hand(hand, horn);
    assert_eq!(
        use_item(&player, &fixture.world, hand),
        InteractionResult::Fail
    );
    assert!(!player.living_base().is_using_item());
    assert_eq!(player.stats(), Vec::new());
    player
        .inventory
        .lock()
        .set_item_in_hand(hand, custom_horn(0.0, 0));
    assert_eq!(
        use_item(&player, &fixture.world, hand),
        InteractionResult::Consume
    );
    assert_eq!(
        player
            .living_base()
            .decrement_active_item_use()
            .expect("zero-duration use stays active")
            .remaining_ticks(),
        -1
    );
    assert_eq!(payloads(&connection, C_COOLDOWN), Vec::<Vec<u8>>::new());
}

#[test]
fn instrument_damage_respects_creative_and_breaks_in_survival() {
    init_globals();
    let fixture = fresh_test_world("instrument_durability");
    for mode in [GameType::Survival, GameType::Creative] {
        for initial_damage in [0, 2] {
            let (player, _) = player(&fixture.world, 0.5);
            player.restore_game_modes(mode, None);
            player.abilities.lock().update_for_game_mode(mode);
            let mut horn = custom_horn(0.13, 2);
            horn.set(MAX_DAMAGE, 4);
            horn.set(DAMAGE, initial_damage);
            player
                .inventory
                .lock()
                .set_item_in_hand(InteractionHand::OffHand, horn);
            assert_eq!(
                use_item(&player, &fixture.world, InteractionHand::OffHand),
                InteractionResult::Consume
            );
            let inventory = player.inventory.lock();
            let stack = inventory.get_item_in_hand(InteractionHand::OffHand);
            let broke = mode == GameType::Survival && initial_damage == 2;
            assert_eq!(stack.is_empty(), broke);
            if !broke {
                assert_eq!(
                    stack.get_damage_value(),
                    initial_damage + if mode == GameType::Creative { 0 } else { 2 }
                );
            }
            assert_eq!(
                player
                    .living_base()
                    .active_item_use()
                    .expect("active offhand horn")
                    .duration(),
                2
            );
            let broken = Stat::new(&vanilla_stat_types::ITEM_BROKEN, &vanilla_items::GOAT_HORN);
            assert_eq!(player.stats().contains(&(broken, 1)), broke);
        }
    }
}

#[test]
fn instrument_sound_excludes_actor_and_respects_direct_sound_range() {
    init_globals();
    let fixture = fresh_test_world("instrument_sound");
    let world = &fixture.world;
    insert_ready_full_chunk(world, ChunkPos::new(0, 0));
    let (actor, actor_connection) = player(world, 0.5);
    let (nearby, near_connection) = player(world, 8.5);
    let (boundary, boundary_connection) = player(world, 16.5);
    for observer in [&actor, &nearby, &boundary] {
        assert!(world.add_player(Arc::clone(observer), ResetReason::InitialJoin));
    }
    for connection in [&actor_connection, &near_connection, &boundary_connection] {
        connection.0.lock().clear();
    }
    let listener = Arc::new(InstrumentListener(SyncMutex::new(Vec::new())));
    world.register_game_event_listener(
        SectionPos::from_block_pos(BlockPos::new(0, 80, 0)),
        Arc::<InstrumentListener>::clone(&listener),
    );
    actor
        .inventory
        .lock()
        .set_item_in_hand(InteractionHand::MainHand, custom_horn(0.5, 0));
    assert_eq!(
        use_item(&actor, world, InteractionHand::MainHand),
        InteractionResult::Consume
    );
    assert_eq!(payloads(&actor_connection, C_SOUND), Vec::<Vec<u8>>::new());
    assert_eq!(
        payloads(&boundary_connection, C_SOUND),
        Vec::<Vec<u8>>::new()
    );
    let sounds = payloads(&near_connection, C_SOUND);
    assert_eq!(sounds.len(), 1);
    let mut cursor = Cursor::new(sounds[0].as_slice());
    assert_eq!(
        SoundEventHolder::read(&mut cursor).expect("sound packet holder"),
        custom_horn(0.5, 0)
            .get(INSTRUMENT)
            .expect("custom horn has an instrument")
            .value()
            .sound_event()
            .clone()
    );
    assert_eq!(VarInt::read(&mut cursor).expect("sound category").0, 2);
    assert_eq!(
        IVec3::read(&mut cursor).expect("sound position"),
        IVec3::new(4, 640, 4)
    );
    assert_eq!(f32::read(&mut cursor).expect("sound volume"), 3.0);
    assert_eq!(f32::read(&mut cursor).expect("sound pitch"), 1.0);
    assert_eq!(
        *listener.0.lock(),
        vec![(
            &vanilla_game_events::INSTRUMENT_PLAY,
            Some(actor.id()),
            actor.position()
        )]
    );
}
