use std::sync::Arc;

use vecta_macros::item_behavior;
use vecta_registry::{
    TaggedRegistryExt as _,
    blocks::{
        BlockRef, block_state_ext::BlockStateExt as _, properties::BlockStateProperties,
        shapes::is_offset_shape_full_block,
    },
    data_components::vanilla_components::USE_EFFECTS,
    item_stack::ItemStack,
    level_events,
    vanilla_biome_tags::BiomeTag,
    vanilla_block_tags::BlockTag,
    vanilla_blocks, vanilla_game_events,
};
use vecta_utils::{BlockPos, BlockStateId, Direction, Identifier, types::UpdateFlags};

use crate::{
    behavior::{BLOCK_BEHAVIORS, InteractionResult, ItemBehavior, UseOnContext},
    entity::Entity,
    world::{LevelRandomSource, LevelReader as _, World},
};
use vecta_utils::random::RandomAccess;

/// Behavior for the Bonemeal item.
#[item_behavior]
pub struct BoneMealItem;

impl BoneMealItem {
    fn cause_finish_use_vibration(context: &UseOnContext<'_>) {
        let interact_vibrations = context.inv.with_item(|item| {
            item.get(USE_EFFECTS)
                .is_some_and(|effects| effects.interact_vibrations)
        });
        if interact_vibrations {
            context
                .player
                .game_event(&vanilla_game_events::ITEM_INTERACT_FINISH);
        }
    }

    /// Vanilla `BoneMealItem.growCrop` without the stack shrink, which the
    /// caller performs once per successful attempt.
    pub(crate) fn grow_crop(world: &Arc<World>, pos: BlockPos) -> bool {
        let state = world.get_block_state(pos);
        let Some(behavior) = BLOCK_BEHAVIORS.get_behavior_for_state(state) else {
            return false;
        };
        let Some(bonemealable) = behavior.as_bonemealable() else {
            return false;
        };
        if !bonemealable.is_valid_bonemeal_target(state, world.as_ref(), pos) {
            return false;
        }

        let mut rng = LevelRandomSource::new(world.random());
        if bonemealable.is_bonemeal_success(state, world, &mut rng, pos) {
            bonemealable.perform_bonemeal(state, world, &mut rng, pos);
        }

        true
    }

    pub(crate) fn grow_water_plant(
        world: &Arc<World>,
        pos: BlockPos,
        clicked_face: Option<Direction>,
    ) -> bool {
        let state = world.get_block_state(pos);
        if state.get_block() != &vanilla_blocks::WATER || !state.get_fluid_state().is_full() {
            return false;
        }

        let Some(seagrass) = BLOCK_BEHAVIORS
            .get_behavior(&vanilla_blocks::SEAGRASS)
            .as_bonemealable()
        else {
            return false;
        };

        let mut rng = LevelRandomSource::new(world.random());

        'outer: for j in 0..128 {
            let mut new_pos = pos;
            let mut new_state = vanilla_blocks::SEAGRASS.default_state();

            for _ in 0..(j / 16) {
                new_pos = new_pos.offset(
                    rng.next_i32_bounded(3) - 1,
                    (rng.next_i32_bounded(3) - 1) * rng.next_i32_bounded(3) / 2,
                    rng.next_i32_bounded(3) - 1,
                );

                if is_offset_shape_full_block(
                    world
                        .get_block_state(new_pos)
                        .get_collision_shape_at(new_pos),
                ) {
                    continue 'outer;
                }
            }

            if world
                .biome_at(new_pos)
                .is_some_and(|biome| biome.has_tag(&BiomeTag::PRODUCES_CORALS_FROM_BONEMEAL))
            {
                if j == 0 && clicked_face.is_some_and(Direction::is_horizontal) {
                    if let Some(coral) = random_tag_block(&BlockTag::WALL_CORALS, &mut rng) {
                        new_state = coral.default_state();
                    }
                    if let Some(face) = clicked_face {
                        new_state = set_facing_if_present(new_state, face);
                    }
                } else if rng.next_i32_bounded(4) == 0
                    && let Some(underwater) =
                        random_tag_block(&BlockTag::UNDERWATER_BONEMEALS, &mut rng)
                {
                    new_state = underwater.default_state();
                }
            }

            if is_wall_coral_state(new_state) {
                let mut retries = 0;
                while !state_can_survive(new_state, world, new_pos) && retries < 4 {
                    let facing = horizontal_direction(&mut rng);
                    new_state = set_facing_if_present(new_state, facing);
                    retries += 1;
                }
            }

            if state_can_survive(new_state, world, new_pos) {
                let current_state = world.get_block_state(new_pos);
                if current_state.get_block() == &vanilla_blocks::WATER
                    && current_state.get_fluid_state().is_full()
                {
                    world.set_block(new_pos, new_state, UpdateFlags::UPDATE_ALL);
                } else if current_state.get_block() == &vanilla_blocks::SEAGRASS
                    && seagrass.is_valid_bonemeal_target(current_state, world.as_ref(), new_pos)
                    && rng.next_i32_bounded(10) == 0
                {
                    seagrass.perform_bonemeal(current_state, world, &mut rng, new_pos);
                }
            }
        }

        true
    }
}

/// Vanilla `Direction.Plane.HORIZONTAL.getRandomDirection(random)`.
fn horizontal_direction(rng: &mut dyn RandomAccess) -> Direction {
    let directions = Direction::HORIZONTAL;
    directions[rng.next_i32_bounded(4) as usize]
}

/// Vanilla `BuiltInRegistries.BLOCK.getRandomElementOf(tag, random)`: one
/// bounded draw over the tag's block list.
fn random_tag_block(tag: &Identifier, rng: &mut dyn RandomAccess) -> Option<BlockRef> {
    let blocks = vecta_registry::REGISTRY.blocks.get_tag(tag)?;
    let count = i32::try_from(blocks.len()).ok()?;
    if count == 0 {
        return None;
    }

    Some(blocks[rng.next_i32_bounded(count) as usize])
}

/// Vanilla `stateToGrow.is(BlockTags.WALL_CORALS, s -> s.hasProperty(FACING))`.
fn is_wall_coral_state(state: BlockStateId) -> bool {
    vecta_registry::REGISTRY
        .blocks
        .is_in_tag(state.get_block(), &BlockTag::WALL_CORALS)
        && state
            .try_get_value(&BlockStateProperties::HORIZONTAL_FACING)
            .is_some()
}

fn set_facing_if_present(state: BlockStateId, facing: Direction) -> BlockStateId {
    if state
        .try_get_value(&BlockStateProperties::HORIZONTAL_FACING)
        .is_some()
    {
        state.set_value(&BlockStateProperties::HORIZONTAL_FACING, facing)
    } else {
        state
    }
}

fn state_can_survive(state: BlockStateId, world: &Arc<World>, pos: BlockPos) -> bool {
    BLOCK_BEHAVIORS
        .get_behavior_for_state(state)
        .is_some_and(|behavior| behavior.can_survive(state, world.as_ref(), pos))
}

impl ItemBehavior for BoneMealItem {
    fn use_on(&self, context: &mut UseOnContext) -> InteractionResult {
        if Self::grow_crop(context.world, context.hit_result.block_pos) {
            context.inv.with_item(ItemStack::shrink_one);
            Self::cause_finish_use_vibration(context);
            context.world.level_event(
                level_events::PARTICLES_AND_SOUND_PLANT_GROWTH,
                context.hit_result.block_pos,
                15,
                None,
            );
            return InteractionResult::Success;
        }
        let state = context.world.get_block_state(context.hit_result.block_pos);
        let is_clicked_face_sturdy = context.world.is_face_sturdy(
            state,
            context.hit_result.block_pos,
            context.hit_result.direction,
        );
        if is_clicked_face_sturdy
            && Self::grow_water_plant(
                context.world,
                context
                    .hit_result
                    .block_pos
                    .relative(context.hit_result.direction),
                Some(context.hit_result.direction),
            )
        {
            context.inv.with_item(ItemStack::shrink_one);
            Self::cause_finish_use_vibration(context);
            context.world.level_event(
                level_events::PARTICLES_AND_SOUND_PLANT_GROWTH,
                context
                    .hit_result
                    .block_pos
                    .relative(context.hit_result.direction),
                15,
                None,
            );
            return InteractionResult::Success;
        }
        InteractionResult::Pass
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use vecta_registry::{REGISTRY, init_vanilla_registry, vanilla_blocks};

    use super::*;

    /// Serves scripted bounded draws and records the bound of every draw.
    #[derive(Default)]
    struct ScriptedRng {
        bounds: Vec<i32>,
        values: VecDeque<i32>,
    }

    impl ScriptedRng {
        fn with_values(values: impl IntoIterator<Item = i32>) -> Self {
            Self {
                bounds: Vec::new(),
                values: values.into_iter().collect(),
            }
        }
    }

    impl RandomAccess for ScriptedRng {
        fn next_i32(&mut self) -> i32 {
            0
        }

        fn next_i32_bounded(&mut self, bound: i32) -> i32 {
            self.bounds.push(bound);
            self.values.pop_front().unwrap_or(0)
        }

        fn next_i64(&mut self) -> i64 {
            0
        }

        fn next_f32(&mut self) -> f32 {
            0.0
        }

        fn next_f64(&mut self) -> f64 {
            0.0
        }

        fn next_bool(&mut self) -> bool {
            false
        }
    }

    fn wall_corals() -> Vec<BlockRef> {
        init_vanilla_registry();
        let Some(corals) = REGISTRY.blocks.get_tag(&BlockTag::WALL_CORALS) else {
            panic!("vanilla must define the wall coral tag");
        };
        corals
    }

    #[test]
    fn a_tag_pick_draws_one_index_over_the_whole_tag() {
        let corals = wall_corals();
        assert!(corals.len() > 1, "the pick must map its index into the tag");
        let Ok(count) = i32::try_from(corals.len()) else {
            panic!("the wall coral tag fits in i32");
        };

        let mut rng = ScriptedRng::with_values([count - 1]);

        let picked = random_tag_block(&BlockTag::WALL_CORALS, &mut rng);

        assert_eq!(picked, Some(corals[corals.len() - 1]));
        assert_eq!(
            rng.bounds,
            [count],
            "vanilla draws once, bounded by the tag size"
        );
    }

    #[test]
    fn the_facing_gate_only_accepts_states_that_carry_the_property() {
        for coral in wall_corals() {
            let state = coral.default_state();
            assert!(
                is_wall_coral_state(state),
                "{} must be a wall coral state with a facing",
                coral.key
            );

            let facing = set_facing_if_present(state, Direction::North);

            assert_eq!(
                facing.get_value(&BlockStateProperties::HORIZONTAL_FACING),
                Direction::North
            );
        }

        let seagrass = vanilla_blocks::SEAGRASS.default_state();
        assert!(!is_wall_coral_state(seagrass));
        assert_eq!(
            set_facing_if_present(seagrass, Direction::North),
            seagrass,
            "states without the facing property stay untouched"
        );
    }

    #[test]
    fn the_horizontal_retry_draw_is_bounded_by_four() {
        let mut rng = ScriptedRng::with_values([2]);

        let direction = horizontal_direction(&mut rng);

        assert_eq!(direction, Direction::HORIZONTAL[2]);
        assert!(direction.is_horizontal());
        assert_eq!(
            rng.bounds,
            [4],
            "vanilla picks one of the four horizontal directions"
        );
    }
}
