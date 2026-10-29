use std::{
    io::Cursor,
    sync::{Arc, Weak},
};

use glam::DVec3;
use text_components::TextComponent;
use vecta_protocol::{
    packet_traits::{CompressionInfo, EncodedPacket},
    packets::game::SSetCreativeModeSlot,
};
use vecta_registry::{
    equipment::EquipmentSlot, item_stack::ItemStack, packets::play::C_SOUND,
    sound_event::SoundEventHolder, vanilla_blocks, vanilla_entities, vanilla_items,
};
use vecta_utils::{
    BlockPos, ChunkPos,
    codec::VarInt,
    locks::SyncMutex,
    serial::ReadFrom as _,
    types::{GameType, InteractionHand, UpdateFlags},
};

use crate::{
    behavior::InteractionResult,
    bootstrap::init_globals,
    entity::{
        Entity, LivingEntity, SharedEntity,
        entities::{
            ChestMinecartEntity, EndCrystalEntity, EndermiteEntity, ExperienceOrbEntity,
            FishingHookEntity, ItemEntity, PigEntity,
        },
        next_entity_id,
    },
    inventory::{
        click::{Click, MouseButton, SwapTarget},
        container::Container as _,
    },
    physics::MoverType,
    player::{
        Player, PlayerConnection, ResetReason, connection::NetworkConnection, game_mode::use_item,
    },
    test_support::{TestPlayerBuilder, fresh_test_world, insert_ready_full_chunk},
    world::World,
};

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

impl RecordingConnection {
    fn sounds(&self) -> Vec<SoundEventHolder> {
        self.0
            .lock()
            .iter()
            .filter_map(|packet| {
                let mut cursor = Cursor::new(packet.encoded_data.as_slice());
                VarInt::read(&mut cursor).expect("packet length should decode");
                let id = VarInt::read(&mut cursor).expect("packet ID should decode");
                (id.0 == C_SOUND).then(|| {
                    SoundEventHolder::read(&mut cursor).expect("sound holder should decode")
                })
            })
            .collect()
    }
}

fn player(world: &Arc<World>) -> (Arc<Player>, Arc<RecordingConnection>) {
    let connection = Arc::new(RecordingConnection::default());
    let player = TestPlayerBuilder::new(Arc::clone(world), "UpstreamFixes", next_entity_id())
        .connection(Arc::new(PlayerConnection::Other(Box::new(Arc::clone(
            &connection,
        )))))
        .build();
    player
        .try_set_position(DVec3::new(0.5, 80.0, 0.5))
        .expect("player should enter loaded chunk");
    player.set_first_tick(false);
    player.set_client_loaded(true);
    assert!(world.add_player(Arc::clone(&player), ResetReason::InitialJoin));
    connection.0.lock().clear();
    (player, connection)
}

#[test]
fn pickup_uses_the_simulated_position_before_the_tick_restores_it() {
    init_globals();
    let fixture = fresh_test_world("upstream_pickup_position");
    let world = &fixture.world;
    insert_ready_full_chunk(world, ChunkPos::new(0, 0));
    let (player, _) = player(world);
    let original_position = player.position();
    let item = Arc::new(ItemEntity::with_item(
        &vanilla_entities::ITEM,
        next_entity_id(),
        original_position + DVec3::new(3.0, 0.0, 0.0),
        ItemStack::new(&vanilla_items::DIAMOND),
        Arc::downgrade(world),
    ));
    let item_entity: SharedEntity = Arc::<ItemEntity>::clone(&item);
    world
        .try_add_entity(item_entity)
        .expect("item should register in the loaded world");
    player.set_velocity(DVec3::new(3.0, 0.0, 0.0));
    let player_entity: SharedEntity = Arc::<Player>::clone(&player);

    player.tick_living_entity(&player_entity);

    assert_eq!(player.position(), original_position);
    assert!(item.is_removed());
    assert!(
        player
            .inventory
            .lock()
            .get_item(0)
            .is(&vanilla_items::DIAMOND)
    );
}

#[test]
fn armor_click_paths_broadcast_one_equip_sound_to_the_actor_and_observer() {
    init_globals();
    for action in 0..3 {
        let fixture = fresh_test_world("upstream_armor_click");
        let world = &fixture.world;
        insert_ready_full_chunk(world, ChunkPos::new(0, 0));
        let (actor, actor_connection) = player(world);
        let (_, observer_connection) = player(world);
        let helmet = ItemStack::new(&vanilla_items::DIAMOND_HELMET);
        let expected = actor
            .equip_sound(EquipmentSlot::Head, &helmet)
            .expect("helmet should have an equip sound");
        let mut menu = actor.inventory_menu.lock();
        let click = if action == 0 {
            *menu.behavior_mut().carried_mut() = helmet;
            Click::Pickup {
                slot: 5,
                button: MouseButton::Left,
            }
        } else {
            actor.inventory.lock().set_item(0, helmet);
            if action == 1 {
                Click::QuickMove { slot: 36 }
            } else {
                Click::Swap {
                    slot: 5,
                    with: SwapTarget::Hotbar(0),
                }
            }
        };

        menu.clicked(click, &actor)
            .expect("armor click should succeed");

        let expected = vec![SoundEventHolder::Registry(expected)];
        assert_eq!(actor_connection.sounds(), expected, "action {action}");
        assert_eq!(observer_connection.sounds(), expected, "action {action}");
    }
}

#[test]
fn creative_equipment_packet_suppresses_identical_replacement_and_unequip_sounds() {
    init_globals();
    let fixture = fresh_test_world("upstream_creative_equip");
    insert_ready_full_chunk(&fixture.world, ChunkPos::new(0, 0));
    let (player, connection) = player(&fixture.world);
    player.restore_game_modes(GameType::Creative, None);
    let helmet = ItemStack::new(&vanilla_items::DIAMOND_HELMET);
    for stack in [helmet.clone(), helmet, ItemStack::empty()] {
        player.handle_set_creative_mode_slot(SSetCreativeModeSlot {
            slot_num: 5,
            item_stack: stack,
        });
        assert_eq!(connection.sounds().len(), 1);
    }
}

#[test]
fn right_click_equipment_uses_the_same_sound_hook() {
    init_globals();
    let fixture = fresh_test_world("upstream_right_click_equip");
    insert_ready_full_chunk(&fixture.world, ChunkPos::new(0, 0));
    let (player, connection) = player(&fixture.world);
    player.inventory.lock().set_item_in_hand(
        InteractionHand::MainHand,
        ItemStack::new(&vanilla_items::DIAMOND_HELMET),
    );

    assert_eq!(
        use_item(&player, &fixture.world, InteractionHand::MainHand),
        InteractionResult::Success
    );
    assert_eq!(connection.sounds().len(), 1);
}

#[test]
fn equipment_sound_hook_respects_first_tick_silence_spectator_and_slot() {
    init_globals();
    let fixture = fresh_test_world("upstream_equip_sound_gates");
    insert_ready_full_chunk(&fixture.world, ChunkPos::new(0, 0));
    let (player, connection) = player(&fixture.world);
    let empty = ItemStack::empty();
    let helmet = ItemStack::new(&vanilla_items::DIAMOND_HELMET);
    player.set_first_tick(true);
    player.on_equip_item(EquipmentSlot::Head, &empty, &helmet);
    player.set_first_tick(false);
    player.set_silent(true);
    player.on_equip_item(EquipmentSlot::Head, &empty, &helmet);
    player.set_silent(false);
    player.restore_game_modes(GameType::Spectator, None);
    player.on_equip_item(EquipmentSlot::Head, &empty, &helmet);
    player.restore_game_modes(GameType::Survival, None);
    player.on_equip_item(EquipmentSlot::Chest, &empty, &helmet);
    assert_eq!(connection.sounds(), Vec::<SoundEventHolder>::new());

    player.on_equip_item(EquipmentSlot::Head, &empty, &helmet);
    assert_eq!(connection.sounds().len(), 1);
}

#[test]
fn movement_does_not_play_steps_for_items_or_other_silent_movers() {
    init_globals();
    let fixture = fresh_test_world("upstream_movement_sounds");
    let world = &fixture.world;
    insert_ready_full_chunk(world, ChunkPos::new(0, 0));
    for x in 0..10 {
        for z in 0..7 {
            world.set_block(
                BlockPos::new(x, 79, z),
                vanilla_blocks::STONE.default_state(),
                UpdateFlags::UPDATE_NONE,
            );
        }
    }
    let (_, connection) = player(world);
    let pos = DVec3::new(2.5, 80.0, 4.5);
    let weak = Arc::downgrade(world);
    let movers: Vec<SharedEntity> = vec![
        Arc::new(ItemEntity::with_item(
            &vanilla_entities::ITEM,
            next_entity_id(),
            pos,
            ItemStack::new(&vanilla_items::STONE),
            Weak::clone(&weak),
        )),
        Arc::new(ExperienceOrbEntity::new(
            &vanilla_entities::EXPERIENCE_ORB,
            next_entity_id(),
            pos,
            Weak::clone(&weak),
        )),
        Arc::new(EndCrystalEntity::new(
            &vanilla_entities::END_CRYSTAL,
            next_entity_id(),
            pos,
            Weak::clone(&weak),
        )),
        Arc::new(FishingHookEntity::new(
            &vanilla_entities::FISHING_BOBBER,
            next_entity_id(),
            pos,
            Weak::clone(&weak),
        )),
        Arc::new(ChestMinecartEntity::new(
            &vanilla_entities::CHEST_MINECART,
            next_entity_id(),
            pos,
            Weak::clone(&weak),
        )),
        Arc::new(EndermiteEntity::new(
            &vanilla_entities::ENDERMITE,
            next_entity_id(),
            pos,
            Weak::clone(&weak),
        )),
    ];
    for entity in movers {
        let name = entity.entity_type();
        let result = entity.move_entity(MoverType::SelfMovement, DVec3::new(4.0, -0.1, 0.0));
        assert!(result.is_some(), "{name:?} should move");
        assert!(
            connection.sounds().is_empty(),
            "{name:?} should not play steps"
        );
    }

    Arc::new(PigEntity::new(
        &vanilla_entities::PIG,
        next_entity_id(),
        pos,
        weak,
    ))
    .move_entity(MoverType::SelfMovement, DVec3::new(4.0, -0.1, 0.0))
    .expect("control pig should move on the same floor");
    assert_ne!(connection.sounds(), Vec::<SoundEventHolder>::new());
}
