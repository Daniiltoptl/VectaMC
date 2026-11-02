//! Bucket item behavior implementation.
//!
//! Handles water buckets, lava buckets, and empty buckets.
//!
//! Mirrors vanilla's `BucketItem(Fluid fluid)`: `fluid_block = None` = empty bucket,
//! `Some(block)` = filled bucket. Logic is dispatched in `use_item`.
//!
use crate::behavior::block::PickupResult;
use crate::behavior::context::InteractionResult;
use crate::behavior::item_utils::create_filled_result;
use crate::behavior::{
    BLOCK_BEHAVIORS, BlockStateBehaviorExt, DispensibleContainerItem, FLUID_BEHAVIORS,
    FluidPlacementHit, ItemBehavior, UseItemContext, pickup_waterlogged_block,
};
use crate::entity::Entity;
use crate::player::Player;
use crate::world::{RaytraceAction, World};
use std::sync::Arc;
use vecta_macros::item_behavior;
use vecta_registry::blocks::BlockRef;
use vecta_registry::blocks::block_state_ext::BlockStateExt;
use vecta_registry::blocks::properties::Direction;
use vecta_registry::fluid::FluidState;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::level_events;
use vecta_registry::sound_events;
use vecta_registry::stat::vanilla_stat_types;
use vecta_registry::vanilla_blocks;
use vecta_registry::vanilla_fluids;
use vecta_registry::vanilla_game_events;
use vecta_registry::vanilla_items;
use vecta_utils::types::UpdateFlags;
use vecta_utils::{BlockPos, BlockStateId};

use crate::world::game_event::GameEventContext;

/// Handles all bucket variants (empty, water, lava).
#[item_behavior]
pub struct BucketItem {
    #[json_arg(vanilla_blocks, json = "content", optional = "empty")]
    fluid_block: Option<BlockRef>,
}

impl BucketItem {
    /// Creates a new bucket behavior. `None` = empty bucket, `Some(block)` = filled.
    #[must_use]
    pub const fn new(fluid_block: Option<BlockRef>) -> Self {
        Self { fluid_block }
    }

    /// Vanilla `BucketItem.emptyContents`: place `fluid_block` at `pos`, retrying
    /// once at the neighbour of the clicked block when the player aimed at a
    /// block that cannot take the fluid.
    ///
    /// Returns whether the fluid was placed.
    fn empty_fluid_contents(
        fluid_block: BlockRef,
        user: Option<&Player>,
        world: &Arc<World>,
        pos: BlockPos,
        hit: Option<FluidPlacementHit>,
    ) -> bool {
        if !world.is_in_valid_bounds(pos) {
            return false;
        }

        let state = world.get_block_state(pos);
        let block = state.get_block();
        let fluid_state = state.get_fluid_state();
        let is_water_bucket = fluid_block == &vanilla_blocks::WATER;
        let behavior = BLOCK_BEHAVIORS.get_behavior(block);
        let is_liquid_container = state.is_liquid_container();
        let may_replace = state.can_be_replaced_by_fluid(fluid_block);
        let can_place_liquid = may_replace
            || (is_liquid_container
                && behavior.can_place_liquid_with_player(
                    state,
                    fluid_block.default_state().get_fluid_state().fluid_id,
                    user,
                ));
        let shift_key_down = user.is_some_and(Player::is_crouching);
        // Vanilla: `blockState.isAir() || placeLiquid && (!shiftKeyDown || hitResult == null)`.
        let can_place_inside =
            block.config.is_air || (can_place_liquid && (!shift_key_down || hit.is_none()));

        if !can_place_inside {
            // Vanilla retries at `hitResult.getBlockPos().relative(getDirection())`;
            // a dispenser passes no hit result and therefore never retries.
            let Some(hit) = hit else {
                return false;
            };
            return Self::empty_fluid_contents(
                fluid_block,
                user,
                world,
                hit.direction.relative(hit.block_pos),
                None,
            );
        }

        // Vanilla parity: in worlds where water evaporates (e.g. the Nether),
        // water buckets fizz out without placing any fluid.
        // TODO: per-position environment attributes (vanilla uses
        // `EnvironmentAttributes.WATER_EVAPORATES` per position) and the
        // `FIRE_EXTINGUISH` sound plus large smoke particles.
        if is_water_bucket && world.dimension_type.water_evaporates {
            world.level_event(level_events::PARTICLES_WATER_EVAPORATING, pos, 0, None);
            return true;
        }

        if is_water_bucket && is_liquid_container {
            behavior.place_liquid(
                world,
                pos,
                state,
                FluidState::source(&vanilla_fluids::WATER),
            );
            play_empty_sound(world, pos, true, user);
            return true;
        }

        // Vanilla destroys a replaceable non-liquid block first so it drops its
        // items (tall grass, flowers, snow layers).
        if may_replace && !block.config.liquid && !block.config.is_air {
            world.destroy_block(pos, true);
        }

        if world.set_block(
            pos,
            fluid_block.default_state(),
            UpdateFlags::UPDATE_ALL_IMMEDIATE,
        ) {
            let fluid_ref = if is_water_bucket {
                &vanilla_fluids::WATER
            } else {
                &vanilla_fluids::LAVA
            };
            let tick_delay = FLUID_BEHAVIORS.get_behavior(fluid_ref).tick_delay(world);
            world.schedule_fluid_tick_default(pos, fluid_ref, tick_delay);
        } else if !fluid_state.is_source() {
            // Vanilla tolerates an unchanged block only when it already holds a
            // source fluid, so placing water on water still empties the bucket.
            return false;
        }

        play_empty_sound(world, pos, is_water_bucket, user);
        true
    }
}

impl DispensibleContainerItem for BucketItem {
    fn empty_contents(
        &self,
        user: Option<&Player>,
        world: &Arc<World>,
        pos: BlockPos,
        hit: Option<FluidPlacementHit>,
    ) -> bool {
        let Some(fluid_block) = self.fluid_block else {
            return false;
        };
        Self::empty_fluid_contents(fluid_block, user, world, pos, hit)
    }
}

impl ItemBehavior for BucketItem {
    fn use_item(&self, context: &mut UseItemContext) -> InteractionResult {
        match self.fluid_block {
            None => use_empty_bucket(context),
            Some(fluid_block) => use_filled_bucket(fluid_block, context),
        }
    }

    fn as_dispensible_container(&self) -> Option<&dyn DispensibleContainerItem> {
        Some(self)
    }
}

fn filled_bucket_success_stack(context: &UseItemContext) -> ItemStack {
    if context.player.has_infinite_materials() {
        context
            .inv
            .with_item(|item| item.copy_with_count(item.count()))
    } else {
        ItemStack::new(&vanilla_items::BUCKET)
    }
}

fn use_empty_bucket(context: &mut UseItemContext) -> InteractionResult {
    let (start, end) = context.player.get_ray_endpoints();

    // Raytrace: stop on source fluids
    let (hit_block, hit_direction) = context.world.raytrace(start, end, |pos, world| {
        let state = world.get_block_state(pos);
        let block = state.get_block();

        if block == &vanilla_blocks::AIR {
            return RaytraceAction::Pass;
        }

        let fluid_state = state.get_fluid_state();
        if fluid_state.is_source() {
            return RaytraceAction::ImmediateHit;
        }
        // Vanilla parity: ClipContext.Fluid.SOURCE_ONLY — flowing fluid is transparent.
        if !fluid_state.is_empty() {
            return RaytraceAction::Pass;
        }

        RaytraceAction::CheckShape
    });

    // Vanilla returns PASS when raytrace misses (allows other handlers to try)
    let (Some(hit_pos), Some(direction)) = (hit_block, hit_direction) else {
        return InteractionResult::Pass;
    };

    let item = context.inv.with_item(|item| item.clone());
    if !context
        .player
        .may_use_item_at(direction.relative(hit_pos), direction, &item)
    {
        return InteractionResult::Fail;
    }

    let hit_state = context.world.get_block_state(hit_pos);
    let Some(result) =
        pickup_bucket_content(context.world, hit_pos, hit_state, Some(context.player))
    else {
        // Vanilla returns FAIL here so the client knows no item change occurred.
        return InteractionResult::Fail;
    };

    context.inv.with_item(|item| {
        context
            .player
            .award_stat(&vanilla_stat_types::ITEM_USED, item.item());
    });
    // Vanilla `BucketItem.use`: `bucketPickupBlock.getPickupSound()...player.playSound(...)`.
    if let Some(sound) = result.sound {
        context.player.play_sound(sound, 1.0, 1.0);
    }

    create_filled_result(context, result.filled_bucket, true);
    context.world.game_event(
        &vanilla_game_events::FLUID_PICKUP,
        hit_pos,
        &GameEventContext::new(Some(context.player.as_ref()), None),
    );

    InteractionResult::Success
}

/// Vanilla `BucketPickup.pickupBlock` for the block at `pos`.
///
/// Shared by the hand path and the dispenser so both agree on which blocks give
/// up their fluid.
pub(crate) fn pickup_bucket_content(
    world: &Arc<World>,
    pos: BlockPos,
    state: BlockStateId,
    player: Option<&Player>,
) -> Option<PickupResult> {
    let behavior = BLOCK_BEHAVIORS.get_behavior(state.get_block());
    behavior
        .pickup_block(world, pos, state, player)
        .or_else(|| {
            // TODO: Remove fallback once all waterloggable blocks implement pickup_block.
            pickup_waterlogged_block(behavior, world, pos, state, player)
        })
}

fn use_filled_bucket(fluid_block: BlockRef, context: &mut UseItemContext) -> InteractionResult {
    // Raytrace to find target block
    let (start, end) = context.player.get_ray_endpoints();
    let (ray_block, ray_dir) = context.world.raytrace(start, end, |pos, world| {
        let state = world.get_block_state(pos);
        let block = state.get_block();
        // Filled buckets use ClipContext.Fluid.NONE: ignore fluid shapes, but
        // still test the block shape of waterlogged/container blocks.
        if block == &vanilla_blocks::AIR {
            return RaytraceAction::Pass;
        }
        RaytraceAction::CheckShape
    });

    // Vanilla returns PASS when raytrace misses (allows other handlers to try)
    let (Some(clicked_pos), Some(direction)) = (ray_block, ray_dir) else {
        return InteractionResult::Pass;
    };

    // If the block is out of bounds, return fail
    if !context.world.is_in_valid_bounds(clicked_pos) {
        return InteractionResult::Fail;
    }

    let item = context.inv.with_item(|item| item.clone());
    if !context
        .player
        .may_use_item_at(direction.relative(clicked_pos), direction, &item)
    {
        return InteractionResult::Fail;
    }

    let clicked_state = context.world.get_block_state(clicked_pos);

    // Vanilla `BucketItem.use` picks the clicked position for a water bucket
    // aimed at a liquid container and the facing neighbour everywhere else.
    let place_pos = filled_bucket_primary_pos(
        clicked_state,
        clicked_pos,
        direction,
        fluid_block == &vanilla_blocks::WATER,
    );
    let hit = FluidPlacementHit {
        block_pos: clicked_pos,
        direction,
    };

    if !BucketItem::empty_fluid_contents(
        fluid_block,
        Some(context.player),
        context.world,
        place_pos,
        Some(hit),
    ) {
        return InteractionResult::Fail;
    }

    // Vanilla awards `ITEM_USED` for the bucket itself, not for the empty
    // container it turns into.
    context
        .player
        .award_stat(&vanilla_stat_types::ITEM_USED, item.item());
    let result_stack = filled_bucket_success_stack(context);
    create_filled_result(context, result_stack, true);
    InteractionResult::Success
}

fn play_empty_sound(
    world: &Arc<World>,
    pos: BlockPos,
    is_water_bucket: bool,
    user: Option<&Player>,
) {
    let sound_event = if is_water_bucket {
        &sound_events::ITEM_BUCKET_EMPTY
    } else {
        &sound_events::ITEM_BUCKET_EMPTY_LAVA
    };
    // Vanilla `BucketItem.playEmptySound`: `level.playSound(user, pos, ...)`
    // excludes the placing player and carries the user into `FLUID_PLACE`.
    world.play_block_sound(sound_event, pos, 1.0, 1.0, user.map(Player::id));
    world.game_event(
        &vanilla_game_events::FLUID_PLACE,
        pos,
        &GameEventContext::new(user.map(|player| player as &dyn Entity), None),
    );
}

fn filled_bucket_primary_pos(
    clicked_state: BlockStateId,
    clicked_pos: BlockPos,
    direction: Direction,
    is_water_bucket: bool,
) -> BlockPos {
    if is_water_bucket && clicked_state.is_liquid_container() {
        clicked_pos
    } else {
        direction.relative(clicked_pos)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        behavior::init_behaviors,
        player::game_mode::use_item,
        test_support::{TestPlayerBuilder, fresh_test_world, insert_ready_full_chunk},
    };
    use glam::DVec3;
    use std::sync::Arc;
    use vecta_registry::{
        RegistryHolderSet,
        data_component_predicate::DataComponentMatchers,
        data_components::{
            AdventureModePredicate, BlockPredicate, vanilla_components::CAN_PLACE_ON,
        },
        init_vanilla_registry,
        item_stack::ItemStack,
        vanilla_blocks, vanilla_items,
    };
    use vecta_utils::{
        BlockPos, ChunkPos,
        types::{GameType, InteractionHand, UpdateFlags},
    };

    use super::*;

    #[test]
    fn filled_water_bucket_targets_non_waterlogged_liquid_container_in_place() {
        init_vanilla_registry();
        init_behaviors();

        let kelp = vanilla_blocks::KELP.default_state();

        assert_eq!(
            filled_bucket_primary_pos(kelp, BlockPos::ZERO, Direction::North, true),
            BlockPos::ZERO
        );
        assert_eq!(
            filled_bucket_primary_pos(kelp, BlockPos::ZERO, Direction::North, false),
            Direction::North.relative(BlockPos::ZERO)
        );
    }

    #[test]
    fn water_bucket_use_places_from_either_hand_and_only_consumes_in_survival() {
        init_vanilla_registry();
        init_behaviors();
        let hit_pos = BlockPos::new(0, 65, 2);
        for game_mode in [GameType::Survival, GameType::Creative] {
            for hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
                let world_fixture = fresh_test_world("water_bucket_use");
                let world = &world_fixture.world;
                insert_ready_full_chunk(world, ChunkPos::from_block_pos(hit_pos));
                assert!(world.set_block(
                    hit_pos,
                    vanilla_blocks::STONE.default_state(),
                    UpdateFlags::UPDATE_ALL,
                ));

                let player = TestPlayerBuilder::new(Arc::clone(world), "BucketUser", 1).build();
                player
                    .try_set_position(DVec3::new(0.5, 64.0, 0.5))
                    .expect("position player");
                player.set_rotation((0.0, 0.0));
                player.restore_game_modes(game_mode, None);
                player.abilities.lock().update_for_game_mode(game_mode);
                player
                    .inventory
                    .lock()
                    .set_item_in_hand(hand, ItemStack::new(&vanilla_items::WATER_BUCKET));

                assert_eq!(
                    use_item(&player, world, hand),
                    InteractionResult::Success,
                    "a water bucket empties onto the clicked face in {game_mode:?}/{hand:?}"
                );

                // The ray enters the north face of `hit_pos`, so vanilla places
                // the fluid at `hitPos.relative(NORTH)`.
                let placed = hit_pos.relative(Direction::North);
                assert_eq!(
                    world.get_block_state(placed),
                    vanilla_blocks::WATER.default_state()
                );

                let inventory = player.inventory.lock();
                let remaining = inventory.get_item_in_hand(hand);
                if game_mode == GameType::Creative {
                    assert!(
                        remaining.is(&vanilla_items::WATER_BUCKET),
                        "`getEmptySuccessItem` keeps the bucket for infinite materials"
                    );
                } else {
                    assert!(
                        remaining.is(&vanilla_items::BUCKET),
                        "survival turns the bucket into an empty one"
                    );
                }
            }
        }
    }

    #[test]
    fn adventure_buckets_require_can_place_on_and_fail_when_denied() {
        init_vanilla_registry();
        init_behaviors();
        let world_fixture = fresh_test_world("adventure_bucket_restriction");
        let world = &world_fixture.world;
        let pos = BlockPos::new(0, 65, 2);
        insert_ready_full_chunk(world, ChunkPos::from_block_pos(pos));
        assert!(world.set_block(
            pos,
            vanilla_blocks::WATER.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));

        let player = TestPlayerBuilder::new(Arc::clone(world), "TestPlayer", 1).build();
        player.base().set_position_local(DVec3::new(0.5, 64.0, 0.5));
        player.set_rotation((0.0, 0.0));
        player.restore_game_modes(GameType::Adventure, None);
        player
            .abilities
            .lock()
            .update_for_game_mode(GameType::Adventure);

        player
            .inventory
            .lock()
            .set_selected_item(ItemStack::new(&vanilla_items::BUCKET));
        assert_eq!(
            use_item(&player, world, InteractionHand::MainHand),
            InteractionResult::Fail
        );
        assert_eq!(
            world.get_block_state(pos),
            vanilla_blocks::WATER.default_state()
        );

        let predicate = BlockPredicate::new(
            Some(RegistryHolderSet::Direct(vec![&vanilla_blocks::WATER])),
            None,
            None,
            DataComponentMatchers::ANY,
        );
        let mut allowed_bucket = ItemStack::new(&vanilla_items::BUCKET);
        allowed_bucket.set(
            CAN_PLACE_ON,
            AdventureModePredicate::new(vec![predicate]).expect("one block predicate is valid"),
        );
        player.inventory.lock().set_selected_item(allowed_bucket);
        assert!(use_item(&player, world, InteractionHand::MainHand).consumes_action());
        assert_eq!(
            world.get_block_state(pos),
            vanilla_blocks::AIR.default_state()
        );

        assert!(world.set_block(
            pos,
            vanilla_blocks::STONE.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        player
            .inventory
            .lock()
            .set_selected_item(ItemStack::new(&vanilla_items::WATER_BUCKET));
        assert_eq!(
            use_item(&player, world, InteractionHand::MainHand),
            InteractionResult::Fail
        );
        assert_eq!(
            world.get_block_state(pos),
            vanilla_blocks::STONE.default_state()
        );
    }
}
