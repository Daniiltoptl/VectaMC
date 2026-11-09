//! Farmland block implementation.

use std::sync::Arc;

use vecta_macros::block_behavior;
use vecta_registry::blocks::BlockRef;
use vecta_registry::blocks::block_state_ext::BlockStateExt;
use vecta_registry::blocks::properties::{BlockStateProperties, IntProperty};
use vecta_registry::fluid::FluidStateExt as _;
use vecta_registry::vanilla_block_tags::BlockTag;
use vecta_registry::{vanilla_blocks, vanilla_game_rules};
use vecta_utils::random::Random;
use vecta_utils::{BlockPos, BlockStateId, Direction, types::UpdateFlags};

use crate::behavior::block::{BlockBehavior, EntityFallDamage, EntityFallOnContext};
use crate::behavior::context::BlockPlaceContext;
use crate::entity::ai::path::PathComputationType;
use crate::world::{LevelReader, ScheduledTickAccess, World};

use super::turn_to_dirt;

const MOISTURE: &IntProperty = &BlockStateProperties::MOISTURE;

/// Maximum moisture level for farmland. Vanilla `FarmlandBlock.MAX_MOISTURE`.
const MAX_MOISTURE: u8 = 7;
/// Vanilla compares the trampling volume in `float`: `> 0.512F`.
const TRAMPLE_VOLUME_THRESHOLD: f32 = 0.512;

/// Behavior for farmland blocks.
///
/// Vanilla parity: `FarmlandBlock` (26.3).
///
/// - `randomTick` hydrates to moisture 7 near water or while it rains above the
///   block, otherwise dries by one step and finally reverts to dirt unless the
///   block above maintains farmland.
/// - `fallOn` tramples the block into dirt for large living entities using the
///   level random.
/// - `updateShape`/`tick`/`canSurvive` handle losing support or cover.
#[block_behavior]
pub struct FarmlandBlock {
    block: BlockRef,
}

impl FarmlandBlock {
    /// Creates a new farmland block behavior.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }

    /// Vanilla `FarmlandBlock.isNearWater`: any water fluid state (source,
    /// flowing, or waterlogged blocks) inside the 9x2x9 box from
    /// `(-4, 0, -4)` to `(4, 1, 4)` relative to the farmland.
    fn is_near_water(world: &dyn LevelReader, pos: BlockPos) -> bool {
        for dy in 0..=1 {
            for dx in -4..=4 {
                for dz in -4..=4 {
                    if world
                        .get_block_state(pos.offset(dx, dy, dz))
                        .get_fluid_state()
                        .is_water()
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Vanilla `shouldMaintainFarmland`: the block above carries the
    /// `MAINTAINS_FARMLAND` block tag (crops, stems, fence gates, pistons).
    fn should_maintain_farmland(world: &dyn LevelReader, pos: BlockPos) -> bool {
        world
            .get_block_state(pos.above())
            .get_block()
            .has_tag(&BlockTag::MAINTAINS_FARMLAND)
    }

    #[must_use]
    fn should_turn_to_dirt_on_fall(
        context: EntityFallOnContext<'_>,
        mob_griefing: bool,
        random_float: f32,
    ) -> bool {
        f64::from(random_float) < context.fall_distance - 0.5
            && context.entity.is_living_entity
            && (context.entity.is_player() || mob_griefing)
            && context.entity.bounding_box_width_squared_height() > TRAMPLE_VOLUME_THRESHOLD
    }
}

impl BlockBehavior for FarmlandBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        // Vanilla: when the default farmland state cannot survive at the
        // clicked position (a solid, non-maintaining block above), the item
        // places its base block (dirt) instead.
        if self.can_survive(
            self.block.default_state(),
            context.world,
            context.place_pos(),
        ) {
            Some(self.block.default_state())
        } else {
            Some(vanilla_blocks::DIRT.default_state())
        }
    }

    fn update_shape(
        &self,
        state: BlockStateId,
        world: &dyn ScheduledTickAccess,
        pos: BlockPos,
        direction: Direction,
        _neighbor_pos: BlockPos,
        _neighbor_state: BlockStateId,
    ) -> BlockStateId {
        if direction == Direction::Up && !self.can_survive(state, world, pos) {
            world.schedule_block_tick_default(pos, self.block, 1);
        }
        state
    }

    fn can_survive(&self, _state: BlockStateId, world: &dyn LevelReader, pos: BlockPos) -> bool {
        // Vanilla: `!aboveState.isSolid() || shouldMaintainFarmland(level, pos)`.
        !world.get_block_state(pos.above()).is_solid() || Self::should_maintain_farmland(world, pos)
    }

    fn is_pathfindable(
        &self,
        _state: BlockStateId,
        _computation_type: PathComputationType,
    ) -> bool {
        // Vanilla FarmlandBlock override: never pathfindable.
        false
    }

    fn tick(&self, state: BlockStateId, world: &Arc<World>, pos: BlockPos) {
        if !self.can_survive(state, world, pos) {
            turn_to_dirt(state, world, pos, None);
        }
    }

    fn random_tick(&self, state: BlockStateId, world: &Arc<World>, pos: BlockPos) {
        let moisture: u8 = state.get_value(MOISTURE);

        if !Self::is_near_water(world, pos) && !world.is_raining_at(pos.above()) {
            if moisture > 0 {
                // Dry out by one step. Vanilla flag 2 = UPDATE_CLIENTS.
                let new_state = state.set_value(MOISTURE, moisture - 1);
                world.set_block(pos, new_state, UpdateFlags::UPDATE_CLIENTS);
            } else if !Self::should_maintain_farmland(world, pos) {
                turn_to_dirt(state, world, pos, None);
            }
        } else if moisture < MAX_MOISTURE {
            let new_state = state.set_value(MOISTURE, MAX_MOISTURE);
            world.set_block(pos, new_state, UpdateFlags::UPDATE_CLIENTS);
        }
    }

    fn fall_on(
        &self,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        context: EntityFallOnContext<'_>,
    ) -> Option<EntityFallDamage> {
        // Vanilla `FarmlandBlock.fallOn` draws `level.getRandom().nextFloat()`
        // on every landing, before any condition short-circuits the `if`.
        let random_float = world.random().with_level(Random::next_f32);
        let mob_griefing = world.get_game_rule(&vanilla_game_rules::MOB_GRIEFING);
        if Self::should_turn_to_dirt_on_fall(context, mob_griefing, random_float) {
            turn_to_dirt(state, world, pos, context.source_entity());
        }

        self.default_fall_on(state, world, pos, context)
    }
}

#[cfg(test)]
mod tests {
    use vecta_registry::item_stack::ItemStack;
    use vecta_registry::sound_events;
    use vecta_registry::vanilla_items;
    use vecta_registry::{init_vanilla_registry, vanilla_entities};
    use vecta_utils::ChunkPos;
    use vecta_utils::types::InteractionHand;

    use crate::behavior::EntityFallOnFacts;
    use crate::behavior::{BlockHitResult, PlacementOrientation, PlacementSource};
    use crate::bootstrap::init_globals;
    use crate::test_support::{TestLevel, fresh_test_world, insert_ready_full_chunk};

    use super::*;

    fn fall_context(
        fall_distance: f64,
        entity_type_is_player: bool,
        is_living_entity: bool,
        bounding_box_width: f64,
        bounding_box_height: f64,
    ) -> EntityFallOnContext<'static> {
        EntityFallOnContext::new(
            fall_distance,
            false,
            EntityFallOnFacts::new(
                if entity_type_is_player {
                    &vanilla_entities::PLAYER
                } else {
                    &vanilla_entities::ZOMBIE
                },
                is_living_entity,
                bounding_box_width,
                bounding_box_height,
                (
                    &sound_events::ENTITY_GENERIC_SMALL_FALL,
                    &sound_events::ENTITY_GENERIC_BIG_FALL,
                ),
            ),
            None,
        )
    }

    #[test]
    fn fall_trampling_requires_random_below_fall_distance_minus_half() {
        assert!(FarmlandBlock::should_turn_to_dirt_on_fall(
            fall_context(1.0, true, true, 0.6, 1.8),
            false,
            0.49,
        ));
        assert!(!FarmlandBlock::should_turn_to_dirt_on_fall(
            fall_context(1.0, true, true, 0.6, 1.8),
            false,
            0.5,
        ));
    }

    #[test]
    fn non_player_living_entities_need_mob_griefing_to_trample() {
        let context = fall_context(1.0, false, true, 0.6, 1.8);

        assert!(!FarmlandBlock::should_turn_to_dirt_on_fall(
            context, false, 0.0,
        ));
        assert!(FarmlandBlock::should_turn_to_dirt_on_fall(
            context, true, 0.0,
        ));
    }

    #[test]
    fn small_or_non_living_entities_do_not_trample() {
        assert!(!FarmlandBlock::should_turn_to_dirt_on_fall(
            fall_context(1.0, true, false, 0.6, 1.8),
            false,
            0.0,
        ));
        assert!(!FarmlandBlock::should_turn_to_dirt_on_fall(
            fall_context(1.0, true, true, 0.25, 0.25),
            false,
            0.0,
        ));
    }

    fn level_with_above(above: BlockStateId) -> TestLevel {
        TestLevel::default().with_block(BlockPos::ZERO.above(), above)
    }

    #[test]
    fn farmland_survives_under_maintaining_blocks_but_not_solid_ones() {
        init_vanilla_registry();

        let farmland = FarmlandBlock::new(&vanilla_blocks::FARMLAND);
        let state = vanilla_blocks::FARMLAND.default_state();

        assert!(farmland.can_survive(
            state,
            &level_with_above(vanilla_blocks::AIR.default_state()),
            BlockPos::ZERO,
        ));
        assert!(!farmland.can_survive(
            state,
            &level_with_above(vanilla_blocks::STONE.default_state()),
            BlockPos::ZERO,
        ));
        // Vanilla MAINTAINS_FARMLAND covers crops and fence gates, even though
        // fence gates are solid.
        assert!(farmland.can_survive(
            state,
            &level_with_above(vanilla_blocks::WHEAT.default_state()),
            BlockPos::ZERO,
        ));
        assert!(farmland.can_survive(
            state,
            &level_with_above(vanilla_blocks::OAK_FENCE_GATE.default_state()),
            BlockPos::ZERO,
        ));
    }

    #[test]
    fn update_shape_schedules_a_survival_tick_only_from_above() {
        init_vanilla_registry();

        let farmland = FarmlandBlock::new(&vanilla_blocks::FARMLAND);
        let state = vanilla_blocks::FARMLAND.default_state();
        let above = BlockPos::ZERO.above();
        let level = level_with_above(vanilla_blocks::STONE.default_state());

        let updated = farmland.update_shape(
            state,
            &level,
            BlockPos::ZERO,
            Direction::Up,
            above,
            vanilla_blocks::STONE.default_state(),
        );
        assert_eq!(updated, state, "updateShape keeps the state unchanged");

        let ticks = level.scheduled_block_tick_list();
        assert_eq!(ticks.len(), 1);
        assert_eq!(ticks[0].pos, BlockPos::ZERO);
        assert_eq!(ticks[0].block, &vanilla_blocks::FARMLAND);
        assert_eq!(ticks[0].delay, 1);
        drop(ticks);

        // A sideways neighbor change schedules nothing.
        let empty = TestLevel::default();
        farmland.update_shape(
            state,
            &empty,
            BlockPos::ZERO,
            Direction::North,
            BlockPos::ZERO.north(),
            vanilla_blocks::STONE.default_state(),
        );
        assert!(
            empty.scheduled_block_tick_list().is_empty(),
            "a sideways neighbour change schedules no tick"
        );

        // From above with air (farmland still survives) nothing is scheduled.
        let airy = level_with_above(vanilla_blocks::AIR.default_state());
        farmland.update_shape(
            state,
            &airy,
            BlockPos::ZERO,
            Direction::Up,
            above,
            vanilla_blocks::AIR.default_state(),
        );
        assert!(
            airy.scheduled_block_tick_list().is_empty(),
            "supported farmland schedules no tick"
        );
    }

    fn farmland_fixture(key: &'static str) -> Arc<World> {
        init_globals();
        let fixture = fresh_test_world(key);
        insert_ready_full_chunk(&fixture.world, ChunkPos::new(0, 0));
        fixture.world
    }

    fn farmland_place_context<'a>(
        world: &'a Arc<World>,
        clicked: BlockPos,
        stack: &'a mut ItemStack,
    ) -> BlockPlaceContext<'a> {
        let source = PlacementSource::direct(
            None,
            InteractionHand::MainHand,
            stack,
            PlacementOrientation::Directional {
                direction: Direction::Up,
            },
            false,
        );
        let hit = BlockHitResult {
            block_pos: clicked,
            direction: Direction::Up,
            location: glam::DVec3::new(
                f64::from(clicked.x()) + 0.5,
                f64::from(clicked.y()) + 1.0,
                f64::from(clicked.z()) + 0.5,
            ),
            miss: false,
            inside: false,
            world_border_hit: false,
        };
        BlockPlaceContext::new(world, source, &hit)
    }

    #[test]
    fn placing_farmland_under_a_solid_block_places_dirt_instead() {
        let world = farmland_fixture("farmland_place_dirt_fallback");

        let clicked = BlockPos::new(8, 64, 8);
        assert!(world.set_block(
            clicked,
            vanilla_blocks::STONE.default_state(),
            UpdateFlags::UPDATE_KNOWN_SHAPE,
        ));
        // A solid cover over the placement position makes farmland unable to
        // survive there.
        assert!(world.set_block(
            clicked.above_n(2),
            vanilla_blocks::STONE.default_state(),
            UpdateFlags::UPDATE_KNOWN_SHAPE,
        ));

        let farmland = FarmlandBlock::new(&vanilla_blocks::FARMLAND);
        let mut stack = ItemStack::new(&vanilla_items::FARMLAND);
        let context = farmland_place_context(&world, clicked, &mut stack);
        assert_eq!(context.place_pos(), clicked.above());

        assert_eq!(
            farmland.get_state_for_placement(&context),
            Some(vanilla_blocks::DIRT.default_state()),
        );
    }

    #[test]
    fn placing_farmland_in_the_open_keeps_default_moisture_zero() {
        let world = farmland_fixture("farmland_place_open");

        let clicked = BlockPos::new(8, 64, 8);
        assert!(world.set_block(
            clicked,
            vanilla_blocks::DIRT.default_state(),
            UpdateFlags::UPDATE_KNOWN_SHAPE,
        ));

        let farmland = FarmlandBlock::new(&vanilla_blocks::FARMLAND);
        let mut stack = ItemStack::new(&vanilla_items::FARMLAND);
        let context = farmland_place_context(&world, clicked, &mut stack);
        assert_eq!(context.place_pos(), clicked.above());

        assert_eq!(
            farmland.get_state_for_placement(&context),
            Some(vanilla_blocks::FARMLAND.default_state()),
        );
    }

    fn hydration_world(key: &'static str, pos: BlockPos, moisture: u8) -> Arc<World> {
        let world = farmland_fixture(key);
        let farmland = vanilla_blocks::FARMLAND
            .default_state()
            .set_value(MOISTURE, moisture);
        assert!(world.set_block(pos, farmland, UpdateFlags::UPDATE_KNOWN_SHAPE));
        world
    }

    #[test]
    fn random_tick_hydrates_farmland_next_to_water_to_full_moisture() {
        let pos = BlockPos::new(8, 64, 8);
        let world = hydration_world("farmland_hydrates", pos, 0);
        assert!(world.set_block(
            pos.east(),
            vanilla_blocks::WATER.default_state(),
            UpdateFlags::UPDATE_KNOWN_SHAPE,
        ));

        FarmlandBlock::new(&vanilla_blocks::FARMLAND).random_tick(
            world.get_block_state(pos),
            &world,
            pos,
        );

        assert_eq!(world.get_block_state(pos).get_value(MOISTURE), MAX_MOISTURE,);
    }

    #[test]
    fn random_tick_dries_farmland_by_one_step_without_water() {
        let pos = BlockPos::new(8, 64, 8);
        let world = hydration_world("farmland_dries", pos, 3);

        FarmlandBlock::new(&vanilla_blocks::FARMLAND).random_tick(
            world.get_block_state(pos),
            &world,
            pos,
        );

        let state = world.get_block_state(pos);
        assert_eq!(state.get_block(), &vanilla_blocks::FARMLAND);
        assert_eq!(state.get_value(MOISTURE), 2);
    }

    #[test]
    fn dry_farmland_without_a_crop_reverts_to_dirt_on_a_random_tick() {
        let pos = BlockPos::new(8, 64, 8);
        let world = hydration_world("farmland_reverts", pos, 0);

        FarmlandBlock::new(&vanilla_blocks::FARMLAND).random_tick(
            world.get_block_state(pos),
            &world,
            pos,
        );

        assert_eq!(
            world.get_block_state(pos),
            vanilla_blocks::DIRT.default_state(),
        );
    }

    #[test]
    fn dry_farmland_under_a_crop_keeps_holding_moisture_zero() {
        let pos = BlockPos::new(8, 64, 8);
        let world = hydration_world("farmland_crop_holds", pos, 0);
        assert!(world.set_block(
            pos.above(),
            vanilla_blocks::WHEAT.default_state(),
            UpdateFlags::UPDATE_KNOWN_SHAPE,
        ));

        FarmlandBlock::new(&vanilla_blocks::FARMLAND).random_tick(
            world.get_block_state(pos),
            &world,
            pos,
        );

        let state = world.get_block_state(pos);
        assert_eq!(state.get_block(), &vanilla_blocks::FARMLAND);
        assert_eq!(state.get_value(MOISTURE), 0);
    }

    #[test]
    fn a_scheduled_survival_tick_reverts_covered_farmland_to_dirt() {
        let pos = BlockPos::new(8, 64, 8);
        let world = hydration_world("farmland_tick_reverts", pos, 0);
        assert!(world.set_block(
            pos.above(),
            vanilla_blocks::STONE.default_state(),
            UpdateFlags::UPDATE_KNOWN_SHAPE,
        ));

        FarmlandBlock::new(&vanilla_blocks::FARMLAND).tick(world.get_block_state(pos), &world, pos);

        assert_eq!(
            world.get_block_state(pos),
            vanilla_blocks::DIRT.default_state(),
        );
    }

    #[test]
    fn a_hard_landing_tramples_farmland_into_dirt_and_breaks_the_crop() {
        let pos = BlockPos::new(8, 64, 8);
        let world = hydration_world("farmland_trample_breaks_crop", pos, 0);
        assert!(world.set_block(
            pos.above(),
            vanilla_blocks::WHEAT.default_state(),
            UpdateFlags::UPDATE_KNOWN_SHAPE,
        ));

        // A fall distance far above the 0.5 threshold makes the vanilla roll
        // deterministic: nextFloat() < fallDistance - 0.5 always holds.
        let context = fall_context(100.0, true, true, 0.6, 1.8);
        let farmland = FarmlandBlock::new(&vanilla_blocks::FARMLAND);
        farmland.fall_on(world.get_block_state(pos), &world, pos, context);

        assert_eq!(
            world.get_block_state(pos),
            vanilla_blocks::DIRT.default_state(),
            "the landing tramples the farmland into dirt",
        );
        assert!(
            world.get_block_state(pos.above()).is_air(),
            "the crop loses its support and pops off",
        );
    }

    #[test]
    fn a_short_landing_never_tramples_farmland() {
        let pos = BlockPos::new(8, 64, 8);
        let world = hydration_world("farmland_no_trample", pos, 5);

        // fallDistance - 0.5 == 0.0: nextFloat() < 0.0 is impossible, so the
        // vanilla roll can never trample regardless of the drawn value.
        let context = fall_context(0.5, true, true, 0.6, 1.8);
        FarmlandBlock::new(&vanilla_blocks::FARMLAND).fall_on(
            world.get_block_state(pos),
            &world,
            pos,
            context,
        );

        let state = world.get_block_state(pos);
        assert_eq!(state.get_block(), &vanilla_blocks::FARMLAND);
        assert_eq!(state.get_value(MOISTURE), 5);
    }
}
