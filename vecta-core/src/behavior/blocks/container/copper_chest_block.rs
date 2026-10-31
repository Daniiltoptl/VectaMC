//! Copper chest behavior.
//!
//! A copper chest is a chest whose two halves always share one oxidation stage:
//! joining two copper chests picks the least oxidized of the pair (and unwaxes
//! both when their wax states differ), and every later state change hands the new
//! block to the partner half. The weathering variant also oxidizes on random
//! ticks, skipping the right half and any chest somebody has open, so a pair can
//! never split into two different ages.

use std::sync::{Arc, Weak};

use vecta_macros::block_behavior;
use vecta_registry::blocks::BlockRef;
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::blocks::properties::{
    BlockStateProperties, ChestType, Direction, EnumProperty,
};
use vecta_registry::sound_event::SoundEventRef;
use vecta_registry::{sound_events, vanilla_block_entity_types, vanilla_custom_stats};
use vecta_utils::{BlockPos, BlockStateId, Downcast as _};

use crate::behavior::blocks::container::chest_block::{ChestBlock, ChestConnect};
use crate::behavior::blocks::{WeatherState, WeatheringCopper};
use crate::behavior::context::{BlockHitResult, BlockPlaceContext, InteractionResult};
use crate::behavior::waxables::get_normal_from_waxed_variant;
use crate::behavior::weathering::get_weather_state;
use crate::behavior::{BlockBehavior, BlockEntityCreation, InventoryAccess};
use crate::block_entity::BLOCK_ENTITIES;
use crate::block_entity::entities::ChestBlockEntity;
use crate::player::Player;
use crate::world::{LevelReader, ScheduledTickAccess, World};

const CHEST_TYPE: &EnumProperty<ChestType> = &BlockStateProperties::CHEST_TYPE;

/// Vanilla `CopperChestBlock` for a waxed variant, which never oxidizes further.
#[block_behavior]
pub struct CopperChestBlock {
    chest: CopperChest,
}

impl CopperChestBlock {
    /// Creates a new copper chest block behavior.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self {
            chest: CopperChest::new(block),
        }
    }
}

/// Vanilla `WeatheringCopperChestBlock`: a copper chest that oxidizes on random
/// ticks.
#[block_behavior]
pub struct WeatheringCopperChestBlock {
    chest: CopperChest,
    #[json_arg(r#enum = "WeatherState", json = "weather_state")]
    weathering: WeatheringCopper,
}

impl WeatheringCopperChestBlock {
    /// Creates a new weathering copper chest block behavior.
    #[must_use]
    pub const fn new(block: BlockRef, weather_state: WeatherState) -> Self {
        Self {
            chest: CopperChest::new(block),
            weathering: WeatheringCopper::new(weather_state),
        }
    }
}

/// The chest behavior every copper chest variant shares.
struct CopperChest {
    block: BlockRef,
}

impl CopperChest {
    const fn new(block: BlockRef) -> Self {
        Self { block }
    }

    /// Vanilla `CopperChestBlock.getStateForPlacement`.
    fn placement_state(&self, context: &BlockPlaceContext<'_>) -> BlockStateId {
        let state = ChestBlock::placement_state(self.block, context, ChestConnect::CopperChests);
        least_oxidized(state, context.world, context.place_pos())
    }

    /// Vanilla `CopperChestBlock.updateShape`: the plain chest join rules, then a
    /// half that has just joined adopts its partner's block type, which is how
    /// both halves end up on the same oxidation stage.
    fn shape_state(
        &self,
        state: BlockStateId,
        world: &dyn ScheduledTickAccess,
        pos: BlockPos,
        direction: Direction,
        neighbor_state: BlockStateId,
    ) -> BlockStateId {
        let shaped = ChestBlock::shape_state(
            self.block,
            state,
            world,
            pos,
            direction,
            neighbor_state,
            ChestConnect::CopperChests,
        );

        if ChestConnect::CopperChests.connects(self.block, neighbor_state)
            && shaped.get_value(CHEST_TYPE) != ChestType::Single
            && ChestBlock::get_connected_direction(shaped) == direction
        {
            return neighbor_state.with_properties_of(shaped);
        }

        shaped
    }

    fn open_at(
        world: &Arc<World>,
        player: &Player,
        state: BlockStateId,
        pos: BlockPos,
    ) -> InteractionResult {
        ChestBlock::open_at(world, player, state, pos, &vanilla_custom_stats::OPEN_CHEST)
    }

    /// Vanilla `CopperChestBlock.getHingeSound`: the weathering stage picks the
    /// sound, and both early stages share the plain copper chest one.
    fn lid_sound(&self, opening: bool) -> SoundEventRef {
        match (weather_state_of(self.block), opening) {
            (Some(WeatherState::Weathered), true) => {
                &sound_events::BLOCK_COPPER_CHEST_WEATHERED_OPEN
            }
            (Some(WeatherState::Weathered), false) => {
                &sound_events::BLOCK_COPPER_CHEST_WEATHERED_CLOSE
            }
            (Some(WeatherState::Oxidized), true) => &sound_events::BLOCK_COPPER_CHEST_OXIDIZED_OPEN,
            (Some(WeatherState::Oxidized), false) => {
                &sound_events::BLOCK_COPPER_CHEST_OXIDIZED_CLOSE
            }
            (_, true) => &sound_events::BLOCK_COPPER_CHEST_OPEN,
            (_, false) => &sound_events::BLOCK_COPPER_CHEST_CLOSE,
        }
    }
}

/// Vanilla `HoneycombItem.WAX_OFF_BY_BLOCK` for a single state.
fn unwax(state: BlockStateId) -> BlockStateId {
    get_normal_from_waxed_variant(state.get_block()).map_or(state, |block| {
        block.default_state().with_properties_of(state)
    })
}

/// Vanilla `CopperChestBlock.isWaxed` for an arbitrary copper chest block.
fn is_waxed(block: BlockRef) -> bool {
    get_normal_from_waxed_variant(block).is_some()
}

/// The oxidation stage of an arbitrary copper chest block, waxed or not.
fn weather_state_of(block: BlockRef) -> Option<WeatherState> {
    get_weather_state(block)
        .or_else(|| get_normal_from_waxed_variant(block).and_then(get_weather_state))
}

/// Vanilla `CopperChestBlock.getLeastOxidizedChestOfConnectedBlocks`: a joined
/// pair is brought to the least oxidized of its halves, unwaxing both first when
/// only one of them is waxed.
fn least_oxidized(state: BlockStateId, level: &dyn LevelReader, pos: BlockPos) -> BlockStateId {
    if state.get_value(CHEST_TYPE) == ChestType::Single {
        return state;
    }

    let connected_state =
        level.get_block_state(pos.relative(ChestBlock::get_connected_direction(state)));
    let Some(own_age) = weather_state_of(state.get_block()) else {
        return state;
    };
    let Some(connected_age) = weather_state_of(connected_state.get_block()) else {
        return state;
    };

    let mut updated = state;
    let mut predicted = connected_state;
    if is_waxed(state.get_block()) != is_waxed(connected_state.get_block()) {
        updated = unwax(updated);
        predicted = unwax(predicted);
    }

    let least = if own_age <= connected_age {
        updated.get_block()
    } else {
        predicted.get_block()
    };
    least.default_state().with_properties_of(updated)
}

impl BlockBehavior for CopperChestBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        Some(self.chest.placement_state(context))
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
        self.chest
            .shape_state(state, world, pos, direction, neighbor_state)
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
        CopperChest::open_at(world, player, state, pos)
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
        copper_chest_block_entity(level, pos, state)
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

    fn affect_neighbors_after_removal(
        &self,
        _state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        _moved_by_piston: bool,
    ) {
        world.update_neighbor_for_output_signal(pos, self.chest.block);
    }

    fn chest_lid_sound(&self, opening: bool) -> Option<SoundEventRef> {
        Some(self.chest.lid_sound(opening))
    }
}

impl BlockBehavior for WeatheringCopperChestBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        Some(self.chest.placement_state(context))
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
        self.chest
            .shape_state(state, world, pos, direction, neighbor_state)
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
        CopperChest::open_at(world, player, state, pos)
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
        copper_chest_block_entity(level, pos, state)
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

    fn affect_neighbors_after_removal(
        &self,
        _state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        _moved_by_piston: bool,
    ) {
        world.update_neighbor_for_output_signal(pos, self.chest.block);
    }

    fn chest_lid_sound(&self, opening: bool) -> Option<SoundEventRef> {
        Some(self.chest.lid_sound(opening))
    }

    fn random_tick(&self, state: BlockStateId, world: &Arc<World>, pos: BlockPos) {
        // Vanilla `WeatheringCopperChestBlock.randomTick` skips the right half,
        // which follows its partner, and any chest that still has viewers.
        if state.get_value(CHEST_TYPE) == ChestType::Right {
            return;
        }
        let Some(block_entity) = world.get_block_entity(pos) else {
            return;
        };
        if block_entity.downcast_ref::<ChestBlockEntity>().is_none() {
            return;
        }
        if ChestBlockEntity::open_count_at(world.as_ref(), pos) != 0 {
            return;
        }
        self.weathering.change_over_time(state, world, pos);
    }
}

/// Vanilla `CopperChestBlock` passes `BlockEntityTypes.CHEST`, so a copper chest
/// is backed by a plain chest block entity.
fn copper_chest_block_entity(
    level: Weak<World>,
    pos: BlockPos,
    state: BlockStateId,
) -> BlockEntityCreation {
    BlockEntityCreation::from_registered_factory(BLOCK_ENTITIES.create(
        &vanilla_block_entity_types::CHEST,
        level,
        pos,
        state,
    ))
}

#[cfg(test)]
mod tests {
    use glam::DVec3;
    use vecta_registry::blocks::properties::{BoolProperty, EnumProperty};
    use vecta_registry::stat::vanilla_stat_types;
    use vecta_registry::{
        RegistryEntry as _, item_stack::ItemStack, vanilla_block_entity_types, vanilla_blocks,
        vanilla_custom_stats, vanilla_items, vanilla_menu_types,
    };
    use vecta_utils::types::{InteractionHand, UpdateFlags};
    use vecta_utils::{BlockPos, ChunkPos};

    use super::*;
    use crate::behavior::BLOCK_BEHAVIORS;
    use crate::behavior::context::BlockHitResult;
    use crate::behavior::context::{BlockPlaceContext, PlacementOrientation, PlacementSource};
    use crate::bootstrap::init_globals;
    use crate::entity::{Entity as _, next_entity_id};
    use crate::inventory::lock::ContainerLockGuard;
    use crate::player::ResetReason;
    use crate::test_support::{
        SentPackets, TestPlayerBuilder, TestWorld, block_events, fresh_test_world,
        insert_ready_full_chunk, opened_screens, played_sounds, recording_connection,
    };

    const TEST_POS: BlockPos = BlockPos::new(4, 64, 4);

    const FACING: &EnumProperty<Direction> = &BlockStateProperties::HORIZONTAL_FACING;
    const WATERLOGGED: &BoolProperty = &BlockStateProperties::WATERLOGGED;

    fn copper_fixture(name: &'static str) -> (TestWorld, Arc<Player>) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        let player = TestPlayerBuilder::new(Arc::clone(&fixture.world), "CopperUser", 1).build();
        player.base().set_position_local(DVec3::new(4.5, 64.0, 4.5));
        (fixture, player)
    }

    fn chest_hit(pos: BlockPos) -> BlockHitResult {
        BlockHitResult {
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
        }
    }

    /// Places `block` the way a player does, through the registered behavior, so
    /// the join rules see a real placement state.
    fn place(
        fixture: &TestWorld,
        pos: BlockPos,
        block: BlockRef,
        facing: Direction,
        secondary_use: bool,
    ) {
        let mut stack = ItemStack::new(&vanilla_items::COPPER_CHEST);
        let source = PlacementSource::direct(
            None,
            InteractionHand::MainHand,
            &mut stack,
            PlacementOrientation::Directional {
                direction: facing.opposite(),
            },
            secondary_use,
        );
        let context = BlockPlaceContext::new(&fixture.world, source, &chest_hit(pos));
        let state = BLOCK_BEHAVIORS
            .get_behavior(block)
            .get_state_for_placement(&context)
            .expect("a copper chest always has a placement state");
        assert!(fixture.world.set_block(pos, state, UpdateFlags::UPDATE_ALL));
    }

    fn use_chest(fixture: &TestWorld, player: &Arc<Player>, pos: BlockPos) -> InteractionResult {
        let state = fixture.world.get_block_state(pos);
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);
        BLOCK_BEHAVIORS
            .get_behavior(state.get_block())
            .use_without_item(
                state,
                &fixture.world,
                pos,
                player,
                &chest_hit(pos),
                &mut inventory,
            )
    }

    fn block_at(fixture: &TestWorld, pos: BlockPos) -> BlockRef {
        fixture.world.get_block_state(pos).get_block()
    }

    fn chest_type_at(fixture: &TestWorld, pos: BlockPos) -> ChestType {
        fixture.world.get_block_state(pos).get_value(CHEST_TYPE)
    }

    #[test]
    fn a_placed_copper_chest_faces_the_player_and_keeps_the_water() {
        let (fixture, _player) = copper_fixture("copper_placement");
        assert!(fixture.world.set_block(
            TEST_POS,
            vanilla_blocks::WATER.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));

        place(
            &fixture,
            TEST_POS,
            &vanilla_blocks::COPPER_CHEST,
            Direction::South,
            false,
        );

        let state = fixture.world.get_block_state(TEST_POS);
        assert_eq!(state.get_value(FACING), Direction::South);
        assert_eq!(state.get_value(CHEST_TYPE), ChestType::Single);
        assert!(
            state.get_value(WATERLOGGED),
            "a copper chest placed in water is waterlogged, like any chest"
        );
    }

    #[test]
    fn two_copper_chests_of_different_ages_join_and_take_the_least_oxidized() {
        // The order matters: placing the more oxidized chest second is the case
        // that only vanilla `getLeastOxidizedChestOfConnectedBlocks` can fix,
        // because the older half is already in the world.
        for (first, second) in [
            (
                &vanilla_blocks::WEATHERED_COPPER_CHEST,
                &vanilla_blocks::COPPER_CHEST,
            ),
            (
                &vanilla_blocks::COPPER_CHEST,
                &vanilla_blocks::WEATHERED_COPPER_CHEST,
            ),
        ] {
            let (fixture, _player) = copper_fixture("copper_join_age");
            place(&fixture, TEST_POS, first, Direction::North, false);
            place(&fixture, TEST_POS.east(), second, Direction::North, false);

            assert_eq!(
                block_at(&fixture, TEST_POS),
                &vanilla_blocks::COPPER_CHEST,
                "the pair takes the fresher half"
            );
            assert_eq!(
                block_at(&fixture, TEST_POS.east()),
                &vanilla_blocks::COPPER_CHEST,
                "the newer chest is downgraded to the fresher age"
            );
            assert_eq!(chest_type_at(&fixture, TEST_POS), ChestType::Left);
            assert_eq!(chest_type_at(&fixture, TEST_POS.east()), ChestType::Right);
        }
    }

    #[test]
    fn joining_a_waxed_chest_with_an_unwaxed_one_unwaxes_both() {
        let (fixture, _player) = copper_fixture("copper_join_wax");
        place(
            &fixture,
            TEST_POS,
            &vanilla_blocks::WAXED_COPPER_CHEST,
            Direction::North,
            false,
        );
        place(
            &fixture,
            TEST_POS.east(),
            &vanilla_blocks::EXPOSED_COPPER_CHEST,
            Direction::North,
            false,
        );

        // Both halves are unwaxed and then brought to the fresher age.
        assert_eq!(block_at(&fixture, TEST_POS), &vanilla_blocks::COPPER_CHEST);
        assert_eq!(
            block_at(&fixture, TEST_POS.east()),
            &vanilla_blocks::COPPER_CHEST
        );
    }

    #[test]
    fn removing_a_copper_half_returns_the_other_to_a_single_chest() {
        let (fixture, _player) = copper_fixture("copper_split");
        place(
            &fixture,
            TEST_POS,
            &vanilla_blocks::COPPER_CHEST,
            Direction::North,
            false,
        );
        place(
            &fixture,
            TEST_POS.east(),
            &vanilla_blocks::COPPER_CHEST,
            Direction::North,
            false,
        );

        assert!(fixture.world.set_block(
            TEST_POS.east(),
            vanilla_blocks::AIR.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));

        assert_eq!(chest_type_at(&fixture, TEST_POS), ChestType::Single);
    }

    #[test]
    fn a_copper_chest_pair_opens_the_six_row_menu_and_awards_the_chest_stat() {
        init_globals();
        let fixture = fresh_test_world("copper_double_menu");
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        place(
            &fixture,
            TEST_POS,
            &vanilla_blocks::COPPER_CHEST,
            Direction::North,
            false,
        );
        place(
            &fixture,
            TEST_POS.east(),
            &vanilla_blocks::COPPER_CHEST,
            Direction::North,
            false,
        );

        let (connection, packets) = recording_connection();
        let player = TestPlayerBuilder::new(
            Arc::clone(&fixture.world),
            "CopperListener",
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
            use_chest(&fixture, &player, TEST_POS),
            InteractionResult::Success
        );

        let screens = opened_screens(&packets);
        assert_eq!(screens.len(), 1, "opening a chest sends one open-screen");
        assert_eq!(
            screens[0].1,
            vanilla_menu_types::GENERIC_9X6.id() as i32,
            "a joined copper pair opens the six-row menu"
        );
        assert_eq!(
            player.get_stat(vanilla_stat_types::CUSTOM.get(&vanilla_custom_stats::OPEN_CHEST)),
            1,
            "vanilla `CopperChestBlock` inherits the plain chest statistic"
        );
    }

    #[test]
    fn a_copper_chest_is_backed_by_a_plain_chest_block_entity() {
        let (fixture, player) = copper_fixture("copper_block_entity");
        place(
            &fixture,
            TEST_POS,
            &vanilla_blocks::COPPER_CHEST,
            Direction::North,
            false,
        );

        let block_entity = fixture
            .world
            .get_block_entity(TEST_POS)
            .expect("a placed copper chest has a block entity");
        assert_eq!(
            block_entity.get_type(),
            &vanilla_block_entity_types::CHEST,
            "vanilla `CopperChestBlock` passes BlockEntityTypes.CHEST"
        );
        let chest = block_entity
            .downcast_ref::<ChestBlockEntity>()
            .expect("the copper chest must create a chest block entity");

        use_chest(&fixture, &player, TEST_POS);
        assert_eq!(chest.open_count(), 1, "a copper chest counts its viewers");
        player.close_container();
        assert_eq!(chest.open_count(), 0);
    }

    /// Opens the chest at [`TEST_POS`] for a listener that receives packets.
    fn listening_fixture(
        name: &'static str,
        block: BlockRef,
    ) -> (TestWorld, Arc<Player>, SentPackets) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        place(&fixture, TEST_POS, block, Direction::North, false);

        let (connection, packets) = recording_connection();
        let player =
            TestPlayerBuilder::new(Arc::clone(&fixture.world), "CopperSound", next_entity_id())
                .connection(connection)
                .build();
        player
            .try_set_position(DVec3::new(4.5, 65.0, 4.5))
            .expect("the chest's chunk is loaded, so the player can stand on it");
        assert!(
            fixture
                .world
                .add_player(Arc::clone(&player), ResetReason::InitialJoin),
            "the listener has to be tracked for the sound packet"
        );
        packets.lock().clear();
        (fixture, player, packets)
    }

    #[test]
    fn the_hinge_sound_follows_the_oxidation_stage() {
        for (block, expected) in [
            (
                &vanilla_blocks::COPPER_CHEST,
                &sound_events::BLOCK_COPPER_CHEST_OPEN,
            ),
            (
                &vanilla_blocks::WEATHERED_COPPER_CHEST,
                &sound_events::BLOCK_COPPER_CHEST_WEATHERED_OPEN,
            ),
            (
                &vanilla_blocks::OXIDIZED_COPPER_CHEST,
                &sound_events::BLOCK_COPPER_CHEST_OXIDIZED_OPEN,
            ),
        ] {
            let (fixture, player, packets) = listening_fixture("copper_hinge_sound", block);

            use_chest(&fixture, &player, TEST_POS);
            fixture.world.run_block_events();

            let sounds = played_sounds(&packets);
            assert_eq!(sounds.len(), 1, "opening plays one hinge sound");
            assert_eq!(
                sounds[0].0, expected.key,
                "vanilla `CopperChestBlock.getHingeSound` picks the stage sound"
            );
            let events = block_events(&packets);
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].1, 1, "the lid event still carries the count");
        }
    }

    /// Ticks `block` at `pos` up to `attempts` times, stopping once the block
    /// changes.
    fn random_tick_until_change(fixture: &TestWorld, pos: BlockPos, attempts: usize) -> bool {
        let original = block_at(fixture, pos);
        for _ in 0..attempts {
            let state = fixture.world.get_block_state(pos);
            BLOCK_BEHAVIORS
                .get_behavior(state.get_block())
                .random_tick(state, &fixture.world, pos);
            if block_at(fixture, pos) != original {
                return true;
            }
        }
        false
    }

    #[test]
    fn a_lone_copper_chest_oxidizes_on_a_random_tick() {
        let (fixture, _player) = copper_fixture("copper_weathering");
        place(
            &fixture,
            TEST_POS,
            &vanilla_blocks::COPPER_CHEST,
            Direction::North,
            false,
        );

        assert!(
            random_tick_until_change(&fixture, TEST_POS, 4000),
            "vanilla `WeatheringCopperChestBlock` oxidizes on random ticks"
        );
        assert_eq!(
            block_at(&fixture, TEST_POS),
            &vanilla_blocks::EXPOSED_COPPER_CHEST
        );
    }

    #[test]
    fn the_right_half_of_a_copper_pair_never_oxidizes_on_its_own() {
        let (fixture, _player) = copper_fixture("copper_weathering_right");
        place(
            &fixture,
            TEST_POS,
            &vanilla_blocks::COPPER_CHEST,
            Direction::North,
            false,
        );
        place(
            &fixture,
            TEST_POS.east(),
            &vanilla_blocks::COPPER_CHEST,
            Direction::North,
            false,
        );
        assert_eq!(chest_type_at(&fixture, TEST_POS.east()), ChestType::Right);

        assert!(
            !random_tick_until_change(&fixture, TEST_POS.east(), 500),
            "vanilla skips the right half, which follows its partner"
        );
        assert_eq!(
            block_at(&fixture, TEST_POS.east()),
            &vanilla_blocks::COPPER_CHEST
        );
    }

    #[test]
    fn an_open_copper_chest_does_not_oxidize() {
        let (fixture, player) = copper_fixture("copper_weathering_open");
        place(
            &fixture,
            TEST_POS,
            &vanilla_blocks::COPPER_CHEST,
            Direction::North,
            false,
        );

        use_chest(&fixture, &player, TEST_POS);
        assert_eq!(
            ChestBlockEntity::open_count_at(fixture.world.as_ref(), TEST_POS),
            1
        );

        assert!(
            !random_tick_until_change(&fixture, TEST_POS, 500),
            "vanilla skips a copper chest that still has viewers"
        );
        assert_eq!(block_at(&fixture, TEST_POS), &vanilla_blocks::COPPER_CHEST);
    }

    #[test]
    fn the_comparator_reads_a_copper_chest() {
        let (fixture, _player) = copper_fixture("copper_comparator");
        place(
            &fixture,
            TEST_POS,
            &vanilla_blocks::COPPER_CHEST,
            Direction::North,
            false,
        );
        let state = fixture.world.get_block_state(TEST_POS);
        let behavior = BLOCK_BEHAVIORS.get_behavior(state.get_block());

        assert_eq!(
            behavior.get_analog_output_signal(state, &fixture.world, TEST_POS, Direction::South),
            0
        );

        let block_entity = fixture
            .world
            .get_block_entity(TEST_POS)
            .expect("a placed copper chest has a block entity");
        let container_ref = block_entity
            .container_ref()
            .expect("a copper chest exposes its container");
        {
            let mut guard = ContainerLockGuard::lock_all(&[&container_ref]);
            guard
                .get_mut(container_ref.container_id())
                .expect("the copper chest container is locked")
                .set_item(0, ItemStack::with_count(&vanilla_items::STONE, 64));
        }

        assert!(
            behavior.get_analog_output_signal(state, &fixture.world, TEST_POS, Direction::South)
                > 0,
            "vanilla `CopperChestBlock` inherits the comparator read"
        );
    }
}
