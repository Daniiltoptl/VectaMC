//! Trapped chest block behavior.
//!
//! A trapped chest is a chest that also emits a redstone signal whose strength
//! is its viewer count, so a comparator or a wire can tell when someone has the
//! lid open. It joins into double chests, opens the same menu and reads the same
//! comparator fullness as a plain chest; the only differences are the stat
//! awarded on opening and the signal it emits.

use std::sync::{Arc, Weak};

use vecta_macros::block_behavior;
use vecta_registry::blocks::BlockRef;
use vecta_registry::blocks::properties::Direction;
use vecta_registry::{vanilla_block_entity_types, vanilla_custom_stats};
use vecta_utils::{BlockPos, BlockStateId};

use crate::behavior::block::BlockBehavior;
use crate::behavior::blocks::container::chest_block::ChestBlock;
use crate::behavior::context::{BlockHitResult, BlockPlaceContext, InteractionResult};
use crate::behavior::{BlockEntityCreation, InventoryAccess};
use crate::block_entity::BLOCK_ENTITIES;
use crate::block_entity::entities::ChestBlockEntity;
use crate::player::Player;
use crate::world::{LevelReader, ScheduledTickAccess, SignalQueryContext, World};

/// Vanilla `TrappedChestBlock`: everything a chest does, plus a signal.
#[block_behavior]
pub struct TrappedChestBlock {
    block: BlockRef,
}

impl TrappedChestBlock {
    /// Creates a new trapped chest block behavior.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }
}

impl BlockBehavior for TrappedChestBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        Some(ChestBlock::placement_state(self.block, context))
    }

    fn update_shape(
        &self,
        state: BlockStateId,
        world: &dyn ScheduledTickAccess,
        pos: BlockPos,
        direction: Direction,
        _neighbor_pos: BlockPos,
        neighbor_state: BlockStateId,
    ) -> BlockStateId {
        ChestBlock::shape_state(self.block, state, world, pos, direction, neighbor_state)
    }

    fn use_without_item(
        &self,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        player: &Player,
        _hit_result: &BlockHitResult,
        _inv: &mut InventoryAccess,
    ) -> InteractionResult {
        // Vanilla `TrappedChestBlock.getOpenChestStat` is the trapped-chest
        // statistic, not the plain chest one.
        ChestBlock::open_at(
            world,
            player,
            state,
            pos,
            &vanilla_custom_stats::TRIGGER_TRAPPED_CHEST,
        )
    }

    fn tick(&self, _state: BlockStateId, world: &Arc<World>, pos: BlockPos) {
        ChestBlock::recheck_at(world, pos);
    }

    fn trigger_event(
        &self,
        _state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        event: i32,
        data: i32,
    ) -> bool {
        ChestBlock::dispatch_block_event(world, pos, event, data)
    }

    fn new_block_entity(
        &self,
        level: Weak<World>,
        pos: BlockPos,
        state: BlockStateId,
    ) -> BlockEntityCreation {
        BlockEntityCreation::from_registered_factory(BLOCK_ENTITIES.create(
            &vanilla_block_entity_types::TRAPPED_CHEST,
            level,
            pos,
            state,
        ))
    }

    fn has_analog_output_signal(&self, _state: BlockStateId) -> bool {
        true
    }

    fn get_analog_output_signal(
        &self,
        state: BlockStateId,
        world: &dyn LevelReader,
        pos: BlockPos,
        _direction: Direction,
    ) -> i32 {
        ChestBlock::analog_output(state, world, pos)
    }

    /// Vanilla `TrappedChestBlock.affectNeighborsAfterRemoval` inherits
    /// `ChestBlock`'s, so a comparator that read this chest re-evaluates.
    fn affect_neighbors_after_removal(
        &self,
        _state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        _moved_by_piston: bool,
    ) {
        world.update_neighbor_for_output_signal(pos, self.block);
    }

    /// Vanilla `TrappedChestBlock.isSignalSource` is always true, unlike a plain
    /// chest, so redstone dust connects to it.
    fn is_signal_source(&self, _state: BlockStateId, _context: SignalQueryContext) -> bool {
        true
    }

    /// Vanilla `TrappedChestBlock.ownSignal`: the viewer count, clamped to the
    /// redstone range.
    fn get_own_signal(
        &self,
        _state: BlockStateId,
        world: &dyn LevelReader,
        pos: BlockPos,
        _context: SignalQueryContext,
    ) -> i32 {
        ChestBlockEntity::open_count_at(world, pos).clamp(0, 15)
    }

    /// Vanilla `TrappedChestBlock.getDirectSignal`: a trapped chest only powers
    /// the block above it directly.
    fn get_direct_signal(
        &self,
        state: BlockStateId,
        world: &dyn LevelReader,
        pos: BlockPos,
        direction: Direction,
        context: SignalQueryContext,
    ) -> i32 {
        if direction == Direction::Up {
            self.get_signal(state, world, pos, direction, context)
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::DVec3;
    use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
    use vecta_registry::blocks::properties::{BlockStateProperties, ChestType, EnumProperty};
    use vecta_registry::stat::vanilla_stat_types;
    use vecta_registry::{
        RegistryEntry as _, item_stack::ItemStack, vanilla_block_entity_types, vanilla_blocks,
        vanilla_custom_stats, vanilla_items, vanilla_menu_types,
    };
    use vecta_utils::types::{InteractionHand, UpdateFlags};
    use vecta_utils::{BlockPos, ChunkPos, Downcast as _};

    use super::*;
    use crate::behavior::BLOCK_BEHAVIORS;
    use crate::behavior::context::BlockHitResult;
    use crate::behavior::context::{BlockPlaceContext, PlacementOrientation, PlacementSource};
    use crate::bootstrap::init_globals;
    use crate::entity::{Entity as _, SharedEntity, next_entity_id};
    use crate::player::ResetReason;
    use crate::test_support::{
        TestPlayerBuilder, TestWorld, fresh_test_world, insert_ready_full_chunk, opened_screens,
        recording_connection,
    };
    use crate::world::SignalGetter as _;

    const TEST_POS: BlockPos = BlockPos::new(4, 64, 4);

    const CHEST_TYPE: &EnumProperty<ChestType> = &BlockStateProperties::CHEST_TYPE;

    fn trapped_fixture(name: &'static str) -> (TestWorld, Arc<Player>) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        assert!(fixture.world.set_block(
            TEST_POS,
            vanilla_blocks::TRAPPED_CHEST.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        let player = TestPlayerBuilder::new(Arc::clone(&fixture.world), "TrappedUser", 1).build();
        player.base().set_position_local(DVec3::new(4.5, 64.0, 4.5));
        (fixture, player)
    }

    fn trapped_hit() -> BlockHitResult {
        BlockHitResult {
            location: DVec3::new(4.5, 64.5, 5.0),
            direction: Direction::South,
            block_pos: TEST_POS,
            miss: false,
            inside: false,
            world_border_hit: false,
        }
    }

    fn use_trapped_chest(
        fixture: &TestWorld,
        player: &Arc<Player>,
        pos: BlockPos,
    ) -> InteractionResult {
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);
        TrappedChestBlock::new(&vanilla_blocks::TRAPPED_CHEST).use_without_item(
            fixture.world.get_block_state(pos),
            &fixture.world,
            pos,
            player,
            &trapped_hit(),
            &mut inventory,
        )
    }

    fn open_count_at(fixture: &TestWorld, pos: BlockPos) -> i32 {
        ChestBlockEntity::open_count_at(fixture.world.as_ref(), pos)
    }

    /// Places a trapped chest the way a player does, so the join rules see a
    /// real placement state and the neighbour gets a real shape update.
    fn place_trapped_chest(
        fixture: &TestWorld,
        pos: BlockPos,
        chest_facing: Direction,
        secondary_use: bool,
    ) {
        let behavior = TrappedChestBlock::new(&vanilla_blocks::TRAPPED_CHEST);
        let mut stack = ItemStack::new(&vanilla_items::TRAPPED_CHEST);
        let source = PlacementSource::direct(
            None,
            InteractionHand::MainHand,
            &mut stack,
            PlacementOrientation::Directional {
                direction: chest_facing.opposite(),
            },
            secondary_use,
        );
        let hit = BlockHitResult {
            location: DVec3::new(
                f64::from(pos.x()) + 0.5,
                f64::from(pos.y()) + 0.5,
                f64::from(pos.z()) + 0.5,
            ),
            direction: Direction::Up,
            block_pos: pos,
            miss: false,
            inside: false,
            world_border_hit: false,
        };
        let context = BlockPlaceContext::new(&fixture.world, source, &hit);
        let state = behavior
            .get_state_for_placement(&context)
            .expect("a trapped chest always has a placement state");
        assert!(fixture.world.set_block(pos, state, UpdateFlags::UPDATE_ALL));
    }

    #[test]
    fn a_trapped_chest_is_a_signal_source_that_a_plain_chest_is_not() {
        init_globals();
        let trapped = TrappedChestBlock::new(&vanilla_blocks::TRAPPED_CHEST);
        let plain = ChestBlock::new(&vanilla_blocks::CHEST);

        assert!(
            trapped.is_signal_source(
                vanilla_blocks::TRAPPED_CHEST.default_state(),
                SignalQueryContext::DEFAULT
            ),
            "vanilla `TrappedChestBlock.isSignalSource` is always true"
        );
        assert!(
            !plain.is_signal_source(
                vanilla_blocks::CHEST.default_state(),
                SignalQueryContext::DEFAULT
            ),
            "vanilla `ChestBlock` is not a signal source"
        );
    }

    #[test]
    fn opening_a_trapped_chest_raises_the_signal_and_closing_clears_it() {
        let (fixture, player) = trapped_fixture("trapped_signal");

        assert_eq!(fixture.world.get_signal(TEST_POS, Direction::North), 0);

        use_trapped_chest(&fixture, &player, TEST_POS);
        assert_eq!(open_count_at(&fixture, TEST_POS), 1);
        assert_eq!(
            fixture.world.get_signal(TEST_POS, Direction::North),
            1,
            "vanilla `TrappedChestBlock.ownSignal` is the viewer count"
        );

        player.close_container();
        assert_eq!(
            fixture.world.get_signal(TEST_POS, Direction::North),
            0,
            "the signal follows the viewer count back down"
        );
    }

    #[test]
    fn a_trapped_chest_powers_the_block_above_directly_and_nothing_else() {
        let (fixture, player) = trapped_fixture("trapped_direct_signal");
        use_trapped_chest(&fixture, &player, TEST_POS);

        assert_eq!(
            fixture.world.get_direct_signal(TEST_POS, Direction::Up),
            1,
            "vanilla `TrappedChestBlock.getDirectSignal` forwards upward"
        );
        for direction in [
            Direction::Down,
            Direction::North,
            Direction::South,
            Direction::East,
            Direction::West,
        ] {
            assert_eq!(
                fixture.world.get_direct_signal(TEST_POS, direction),
                0,
                "only the block above is powered directly, not {direction:?}"
            );
        }
    }

    /// Places an unlit lamp next to the trapped chest.
    fn place_lamp(fixture: &TestWorld) {
        assert!(fixture.world.set_block(
            TEST_POS.east(),
            vanilla_blocks::REDSTONE_LAMP.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
    }

    fn lamp_lit(fixture: &TestWorld) -> bool {
        fixture
            .world
            .get_block_state(TEST_POS.east())
            .get_value(&BlockStateProperties::LIT)
    }

    #[test]
    fn opening_a_trapped_chest_lights_the_lamp_beside_it() {
        // `TrappedChestBlockEntity.signalOpenCount` calls `updateNeighborsAt`, so
        // the lamp learns about the new signal without any other block change.
        let (fixture, player) = trapped_fixture("trapped_lamp_on");
        place_lamp(&fixture);
        assert!(!lamp_lit(&fixture), "a fresh lamp starts dark");

        use_trapped_chest(&fixture, &player, TEST_POS);

        assert!(
            lamp_lit(&fixture),
            "the chest must notify its neighbours when its viewer count changes"
        );
    }

    #[test]
    fn closing_a_trapped_chest_lets_the_lamp_go_out_on_the_next_tick() {
        let (fixture, player) = trapped_fixture("trapped_lamp_off");
        place_lamp(&fixture);
        use_trapped_chest(&fixture, &player, TEST_POS);
        assert!(lamp_lit(&fixture));

        player.close_container();
        assert!(
            lamp_lit(&fixture),
            "vanilla `RedstoneLampBlock` turns off after a delay, not instantly"
        );
        assert!(!fixture.world.has_neighbor_signal(TEST_POS.east()));

        let lamp_pos = TEST_POS.east();
        let lamp_state = fixture.world.get_block_state(lamp_pos);
        BLOCK_BEHAVIORS
            .get_behavior(&vanilla_blocks::REDSTONE_LAMP)
            .tick(lamp_state, &fixture.world, lamp_pos);

        assert!(!lamp_lit(&fixture), "the delayed tick clears the lamp");
    }

    #[test]
    fn a_trapped_chest_awards_the_trapped_chest_stat() {
        let (fixture, player) = trapped_fixture("trapped_stat");

        use_trapped_chest(&fixture, &player, TEST_POS);

        assert_eq!(
            player.get_stat(
                vanilla_stat_types::CUSTOM.get(&vanilla_custom_stats::TRIGGER_TRAPPED_CHEST)
            ),
            1,
            "vanilla `TrappedChestBlock.getOpenChestStat` is TRIGGER_TRAPPED_CHEST"
        );
        assert_eq!(
            player.get_stat(vanilla_stat_types::CUSTOM.get(&vanilla_custom_stats::OPEN_CHEST)),
            0,
            "opening a trapped chest is not the plain chest statistic"
        );
    }

    #[test]
    fn a_trapped_chest_never_joins_a_plain_chest() {
        init_globals();
        let fixture = fresh_test_world("trapped_no_join");
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));

        // Vanilla `ChestBlock.chestCanConnectTo` is `blockState.is(this)`, so a
        // trapped chest beside a plain chest leaves both single.
        assert!(fixture.world.set_block(
            TEST_POS,
            vanilla_blocks::CHEST.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        place_trapped_chest(&fixture, TEST_POS.east(), Direction::North, false);

        assert_eq!(
            fixture
                .world
                .get_block_state(TEST_POS)
                .get_value(CHEST_TYPE),
            ChestType::Single
        );
        assert_eq!(
            fixture
                .world
                .get_block_state(TEST_POS.east())
                .get_value(CHEST_TYPE),
            ChestType::Single
        );
    }

    #[test]
    fn two_trapped_chests_join_into_a_six_row_menu() {
        init_globals();
        let fixture = fresh_test_world("trapped_double_menu");
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        place_trapped_chest(&fixture, TEST_POS, Direction::North, false);
        place_trapped_chest(&fixture, TEST_POS.east(), Direction::North, false);
        assert_eq!(
            fixture
                .world
                .get_block_state(TEST_POS.east())
                .get_value(CHEST_TYPE),
            ChestType::Right
        );

        let (connection, packets) = recording_connection();
        let player = TestPlayerBuilder::new(
            Arc::clone(&fixture.world),
            "TrappedListener",
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
            "the listener has to be tracked for the open-screen packet"
        );
        packets.lock().clear();

        assert_eq!(
            use_trapped_chest(&fixture, &player, TEST_POS),
            InteractionResult::Success
        );

        let screens = opened_screens(&packets);
        assert_eq!(screens.len(), 1, "opening a chest sends one open-screen");
        assert_eq!(
            screens[0].1,
            vanilla_menu_types::GENERIC_9X6.id() as i32,
            "two joined trapped chests open the six-row menu"
        );
    }

    #[test]
    fn a_trapped_chest_saves_its_own_block_entity_type() {
        let (fixture, _player) = trapped_fixture("trapped_type");
        let block_entity = fixture
            .world
            .get_block_entity(TEST_POS)
            .expect("a placed trapped chest has a block entity");

        assert_eq!(
            block_entity.get_type(),
            &vanilla_block_entity_types::TRAPPED_CHEST,
            "vanilla `TrappedChestBlockEntity` reports the trapped chest type"
        );
    }

    #[test]
    fn a_trapped_chest_counts_its_viewer_on_the_shared_counter() {
        let (fixture, player) = trapped_fixture("trapped_counter");
        let block_entity = fixture
            .world
            .get_block_entity(TEST_POS)
            .expect("a placed trapped chest has a block entity");
        let chest = block_entity
            .downcast_ref::<ChestBlockEntity>()
            .expect("the trapped chest block must create a chest block entity");

        assert_eq!(chest.open_count(), 0);
        use_trapped_chest(&fixture, &player, TEST_POS);
        assert_eq!(chest.open_count(), 1);
        player.close_container();
        assert_eq!(chest.open_count(), 0);
    }

    #[test]
    fn a_recheck_of_a_trapped_chest_keeps_a_viewer_that_is_still_there() {
        let (fixture, player) = trapped_fixture("trapped_recheck");
        let player_arc: Arc<Player> = Arc::clone(&player);
        let entity: SharedEntity = player_arc;
        fixture
            .world
            .try_add_entity(entity)
            .expect("the test chunk is loaded, so the player registers");
        use_trapped_chest(&fixture, &player, TEST_POS);

        TrappedChestBlock::new(&vanilla_blocks::TRAPPED_CHEST).tick(
            fixture.world.get_block_state(TEST_POS),
            &fixture.world,
            TEST_POS,
        );

        assert_eq!(open_count_at(&fixture, TEST_POS), 1);
        assert_eq!(fixture.world.get_signal(TEST_POS, Direction::North), 1);
    }

    #[test]
    fn a_trapped_chest_with_no_viewers_emits_nothing() {
        let (fixture, _player) = trapped_fixture("trapped_idle");
        assert_eq!(
            fixture.world.get_signal(TEST_POS, Direction::North),
            0,
            "a chest nobody has open is not a power source"
        );
        assert_eq!(fixture.world.get_direct_signal(TEST_POS, Direction::Up), 0);
    }
}
