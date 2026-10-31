//! Ender chest block behavior implementation.

use std::sync::{Arc, Weak};

use text_components::TextComponent;
use vecta_macros::block_behavior;
use vecta_registry::blocks::BlockRef;
use vecta_registry::blocks::block_state_ext::BlockStateExt;
use vecta_registry::blocks::properties::BlockStateProperties;
use vecta_registry::{vanilla_block_entity_types, vanilla_custom_stats};
use vecta_utils::{BlockPos, BlockStateId, Downcast as _, translations};

use crate::behavior::InventoryAccess;
use crate::behavior::block::{BlockBehavior, BlockEntityCreation};
use crate::behavior::context::{BlockHitResult, BlockPlaceContext, InteractionResult};
use crate::block_entity::BLOCK_ENTITIES;
use crate::block_entity::entities::EnderChestBlockEntity;
use crate::inventory::menu::kinds::ender_chest;
use crate::player::Player;
use crate::world::{World, is_redstone_conductor};

/// The ender chest block behavior.
#[block_behavior]
pub struct EnderChestBlock {
    block: BlockRef,
}

impl EnderChestBlock {
    /// Creates a new ender chest block.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }
}

impl BlockBehavior for EnderChestBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        let facing = context.horizontal_direction().opposite();

        Some(
            self.block
                .default_state()
                .set_value(&BlockStateProperties::FACING, facing)
                .set_value(
                    &BlockStateProperties::WATERLOGGED,
                    context.is_water_source(),
                ),
        )
    }

    fn use_without_item(
        &self,
        _state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        player: &Player,
        _hit_result: &BlockHitResult,
        _inv: &mut InventoryAccess,
    ) -> InteractionResult {
        let Some(block_entity) = world.get_block_entity(pos) else {
            return InteractionResult::Success;
        };
        if block_entity.get_type() != &vanilla_block_entity_types::ENDER_CHEST {
            return InteractionResult::Success;
        }

        let above_pos = pos.above();
        if is_redstone_conductor(world.as_ref(), world.get_block_state(above_pos), above_pos) {
            return InteractionResult::Success;
        }

        let inventory = Arc::clone(&player.inventory);
        let container = Arc::clone(&player.ender_chest_inventory);
        let chest = Arc::downgrade(&block_entity);
        player.open_menu(
            TextComponent::translated(translations::CONTAINER_ENDERCHEST.msg()),
            move |context| {
                // open only after active chest has closed, since
                // closing an ender chest menu clears the active chest.
                container.lock().set_active_chest(chest);
                // `ChestMenu` calls `container.startOpen(player)` in its
                // constructor, which the player's storage forwards to the chest.
                container.lock().start_open(context.player);
                ender_chest(inventory, context.container_id, container)
            },
        );

        player.award_custom_stat(&vanilla_custom_stats::OPEN_ENDERCHEST);
        // TODO: Anger nearby piglins

        InteractionResult::Success
    }

    fn new_block_entity(
        &self,
        level: Weak<World>,
        pos: BlockPos,
        state: BlockStateId,
    ) -> BlockEntityCreation {
        BlockEntityCreation::from_registered_factory(BLOCK_ENTITIES.create(
            &vanilla_block_entity_types::ENDER_CHEST,
            level,
            pos,
            state,
        ))
    }

    fn tick(&self, _state: BlockStateId, world: &Arc<World>, pos: BlockPos) {
        // Vanilla `EnderChestBlock.tick` recounts the viewers of an open chest.
        let Some(block_entity) = world.get_block_entity(pos) else {
            return;
        };
        if let Some(ender_chest) = block_entity.downcast_ref::<EnderChestBlockEntity>() {
            ender_chest.recheck_open();
        }
    }

    /// Vanilla `BaseEntityBlock.triggerEvent`, which `EnderChestBlock` inherits:
    /// the block entity decides whether the event is published to clients.
    fn trigger_event(
        &self,
        _state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        event: i32,
        data: i32,
    ) -> bool {
        let Some(block_entity) = world.get_block_entity(pos) else {
            return false;
        };
        block_entity.trigger_event(event, data)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use glam::{DVec3, IVec3};
    use vecta_protocol::packets::game::SoundSource;
    use vecta_registry::blocks::properties::Direction;
    use vecta_registry::item_stack::ItemStack;
    use vecta_registry::{RegistryEntry as _, sound_events, vanilla_blocks, vanilla_items};
    use vecta_utils::types::{InteractionHand, UpdateFlags};
    use vecta_utils::{BlockPos, ChunkPos};

    use super::*;
    use crate::block_entity::SharedBlockEntity;
    use crate::bootstrap::init_globals;
    use crate::entity::{Entity as _, SharedEntity, next_entity_id};
    use crate::inventory::container::Container as _;
    use crate::player::ResetReason;
    use crate::test_support::{
        SentPackets, TestPlayerBuilder, TestWorld, block_events, fresh_test_world,
        insert_ready_full_chunk, played_sounds, recording_connection,
    };

    const TEST_POS: BlockPos = BlockPos::new(4, 64, 4);
    /// `EnderChestBlockEntity` plays its lid sound at the block centre.
    const BLOCK_CENTRE_SOUND_POS: IVec3 = IVec3::new(36, 516, 36);

    fn ender_chest_fixture(name: &'static str) -> (TestWorld, Arc<Player>) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        assert!(fixture.world.set_block(
            TEST_POS,
            vanilla_blocks::ENDER_CHEST.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        let player = TestPlayerBuilder::new(Arc::clone(&fixture.world), "EnderUser", 1).build();
        player.base().set_position_local(DVec3::new(4.5, 64.0, 4.5));
        (fixture, player)
    }

    fn ender_chest_hit() -> BlockHitResult {
        BlockHitResult {
            location: DVec3::new(4.5, 64.5, 5.0),
            direction: Direction::South,
            block_pos: TEST_POS,
            miss: false,
            inside: false,
            world_border_hit: false,
        }
    }

    fn use_ender_chest(fixture: &TestWorld, player: &Arc<Player>) -> InteractionResult {
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);
        EnderChestBlock::new(&vanilla_blocks::ENDER_CHEST).use_without_item(
            fixture.world.get_block_state(TEST_POS),
            &fixture.world,
            TEST_POS,
            player,
            &ender_chest_hit(),
            &mut inventory,
        )
    }

    fn ender_chest(fixture: &TestWorld) -> SharedBlockEntity {
        fixture
            .world
            .get_block_entity(TEST_POS)
            .expect("a placed ender chest has a block entity")
    }

    fn open_count(fixture: &TestWorld) -> i32 {
        ender_chest(fixture)
            .downcast_ref::<EnderChestBlockEntity>()
            .expect("the ender chest block must create its block entity")
            .open_count()
    }

    fn register_player(fixture: &TestWorld, player: &Arc<Player>) {
        let player_arc: Arc<Player> = Arc::clone(player);
        let entity: SharedEntity = player_arc;
        fixture
            .world
            .try_add_entity(entity)
            .expect("the test chunk is loaded, so the player registers");
    }

    fn listening_ender_chest_fixture(name: &'static str) -> (TestWorld, Arc<Player>, SentPackets) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        assert!(fixture.world.set_block(
            TEST_POS,
            vanilla_blocks::ENDER_CHEST.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        let (connection, packets) = recording_connection();
        let player = TestPlayerBuilder::new(
            Arc::clone(&fixture.world),
            "EnderListener",
            next_entity_id(),
        )
        .connection(connection)
        .build();
        player
            .try_set_position(DVec3::new(4.5, 65.0, 4.5))
            .expect("the chest's chunk is loaded, so the player can stand on it");
        assert!(
            fixture
                .world
                .add_player(Arc::clone(&player), ResetReason::InitialJoin),
            "the listener has to be tracked for the sound packet to reach it"
        );
        packets.lock().clear();
        (fixture, player, packets)
    }

    #[test]
    fn opening_an_ender_chest_counts_the_viewer_and_plays_the_lid_sound() {
        let (fixture, player, packets) = listening_ender_chest_fixture("ender_chest_open");

        assert_eq!(
            use_ender_chest(&fixture, &player),
            InteractionResult::Success
        );

        assert_eq!(open_count(&fixture), 1);
        let sounds = played_sounds(&packets);
        assert_eq!(sounds.len(), 1, "opening plays exactly one lid sound");
        assert_eq!(sounds[0].0, sound_events::BLOCK_ENDER_CHEST_OPEN.key);
        assert_eq!(sounds[0].1, SoundSource::Blocks.as_varint());
        assert_eq!(sounds[0].2, BLOCK_CENTRE_SOUND_POS);
        assert!((sounds[0].3 - 0.5).abs() < f32::EPSILON);

        // `openerCountChanged` sends `blockEvent(1, current)`, which is what the
        // client animates the lid from.
        fixture.world.run_block_events();
        let events = block_events(&packets);
        assert_eq!(events.len(), 1, "opening sends exactly one block event");
        assert_eq!(events[0].0, TEST_POS);
        assert_eq!(events[0].1, 1, "event id 1 is the lid event");
        assert_eq!(events[0].2, 1, "the parameter carries the viewer count");
        assert_eq!(events[0].3, vanilla_blocks::ENDER_CHEST.id() as i32);
    }

    #[test]
    fn closing_the_menu_stops_the_count_and_plays_the_close_sound() {
        let (fixture, player, packets) = listening_ender_chest_fixture("ender_chest_close");
        assert_eq!(
            use_ender_chest(&fixture, &player),
            InteractionResult::Success
        );
        assert!(
            player
                .ender_chest_inventory
                .lock()
                .is_active_chest(&*ender_chest(&fixture))
        );
        packets.lock().clear();

        player.close_container();

        assert_eq!(open_count(&fixture), 0);
        let sounds = played_sounds(&packets);
        assert_eq!(sounds.len(), 1, "closing plays exactly one lid sound");
        assert_eq!(sounds[0].0, sound_events::BLOCK_ENDER_CHEST_CLOSE.key);
        assert_eq!(sounds[0].2, BLOCK_CENTRE_SOUND_POS);
        assert!(
            !player
                .ender_chest_inventory
                .lock()
                .is_active_chest(&*ender_chest(&fixture)),
            "`PlayerEnderChestContainer.stopOpen` forgets the active chest"
        );
    }

    #[test]
    fn the_block_tick_recounts_the_viewers() {
        let (fixture, player) = ender_chest_fixture("ender_chest_recheck_tick");
        register_player(&fixture, &player);
        assert_eq!(
            use_ender_chest(&fixture, &player),
            InteractionResult::Success
        );
        assert_eq!(open_count(&fixture), 1);

        // The menu is still open, but the player walked out of reach, which is
        // exactly the case the scheduled recount exists for.
        player
            .base()
            .try_set_position(DVec3::new(15.5, 64.0, 15.5))
            .expect("the player walks out of range");
        EnderChestBlock::new(&vanilla_blocks::ENDER_CHEST).tick(
            fixture.world.get_block_state(TEST_POS),
            &fixture.world,
            TEST_POS,
        );

        assert_eq!(open_count(&fixture), 0);
    }

    #[test]
    fn each_player_keeps_their_own_storage() {
        let (fixture, first) = ender_chest_fixture("ender_chest_storage");
        let second = TestPlayerBuilder::new(Arc::clone(&fixture.world), "EnderSecond", 2).build();
        second.base().set_position_local(DVec3::new(4.5, 64.0, 4.5));

        assert_eq!(
            use_ender_chest(&fixture, &first),
            InteractionResult::Success
        );
        first
            .ender_chest_inventory
            .lock()
            .set_item(0, ItemStack::with_count(&vanilla_items::DIAMOND, 3));
        first.close_container();

        assert_eq!(
            use_ender_chest(&fixture, &second),
            InteractionResult::Success
        );
        let second_item = second.ender_chest_inventory.lock().get_item(0).clone();
        assert!(
            second_item.is_empty(),
            "the storage belongs to the player, not to the block"
        );

        let first_item = first.ender_chest_inventory.lock().get_item(0).clone();
        assert!(first_item.is(&vanilla_items::DIAMOND));
        assert_eq!(first_item.count(), 3);
    }
}
