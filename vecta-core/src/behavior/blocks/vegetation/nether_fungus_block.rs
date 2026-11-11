use std::sync::{Arc, LazyLock};

use vecta_macros::block_behavior;
use vecta_registry::REGISTRY;
use vecta_registry::blocks::block_state_ext::BlockStateExt;
use vecta_registry::feature::{ConfiguredFeature, ConfiguredFeatureKind};
use vecta_utils::random::worldgen_random::WorldgenRandom;
use vecta_utils::{BlockPos, BlockStateId, Identifier};

use crate::behavior::block::BlockBehavior;
use crate::behavior::blocks::vegetation::bonemealable::Bonemealable;
use crate::behavior::context::BlockPlaceContext;
use crate::world::{LevelReader, World};
use crate::worldgen::feature::FeatureDecorationRunner;

use super::{BlockRef, default_surviving_state, survives_on_tag};
use vecta_utils::random::RandomAccess;

const BONEMEAL_SUCCESS_CHANCE: f32 = 0.4;

/// Vanilla `NetherFungusBlock` survival.
#[block_behavior]
pub struct NetherFungusBlock {
    block: BlockRef,
    #[json_arg(vanilla_blocks)]
    required_block: BlockRef,
    #[json_arg(vanilla_configured_features, json = "feature")]
    feature: &'static LazyLock<ConfiguredFeature>,
    #[json_arg(vanilla_block_tags)]
    support_blocks: Identifier,
}

impl NetherFungusBlock {
    /// Creates a new nether fungus block behavior.
    #[must_use]
    pub const fn new(
        block: BlockRef,
        required_block: BlockRef,
        feature: &'static LazyLock<ConfiguredFeature>,
        support_blocks: Identifier,
    ) -> Self {
        Self {
            block,
            required_block,
            feature,
            support_blocks,
        }
    }
}

impl BlockBehavior for NetherFungusBlock {
    fn can_survive(&self, _state: BlockStateId, world: &dyn LevelReader, pos: BlockPos) -> bool {
        survives_on_tag(world, pos, &self.support_blocks)
    }

    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        default_surviving_state(self.block, self, context)
    }

    fn as_bonemealable(&self) -> Option<&dyn Bonemealable> {
        Some(self)
    }
}

impl Bonemealable for NetherFungusBlock {
    fn is_valid_bonemeal_target(
        &self,
        _state: BlockStateId,
        world: &dyn LevelReader,
        pos: BlockPos,
    ) -> bool {
        world.get_block_state(pos.below()).get_block() == self.required_block
            && !world.is_outside_build_height(pos.above().y())
    }

    fn is_bonemeal_success(
        &self,
        _state: BlockStateId,
        _world: &Arc<World>,
        rng: &mut dyn RandomAccess,
        _pos: BlockPos,
    ) -> bool {
        rng.next_f32() < BONEMEAL_SUCCESS_CHANCE
    }

    fn perform_bonemeal(
        &self,
        _state: BlockStateId,
        world: &Arc<World>,
        rng: &mut dyn RandomAccess,
        pos: BlockPos,
    ) {
        let ConfiguredFeatureKind::HugeFungus(config) = &self.feature.kind else {
            return;
        };
        let mut worldgen_random = WorldgenRandom::from_seed(rng.next_i64() as u64);
        FeatureDecorationRunner::place_planted_huge_fungus_feature(
            world,
            &REGISTRY,
            &mut worldgen_random,
            config,
            pos,
        );
    }
}

#[cfg(test)]
mod tests {
    use RandomAccess;
    use rand::{RngExt, SeedableRng, rngs::StdRng};
    use vecta_registry::{
        init_vanilla_registry, vanilla_block_tags::BlockTag, vanilla_blocks,
        vanilla_configured_features,
    };
    use vecta_utils::{ChunkPos, types::UpdateFlags};

    use crate::{
        behavior::init_behaviors,
        test_support::{TestLevel, fresh_test_world, insert_ready_full_chunk},
    };

    use super::*;

    struct FixedRng(u64);

    impl RandomAccess for FixedRng {
        fn next_i32(&mut self) -> i32 {
            self.0 as i32
        }

        fn next_i32_bounded(&mut self, _bound: i32) -> i32 {
            0
        }

        fn next_i64(&mut self) -> i64 {
            self.0 as i64
        }

        fn next_f32(&mut self) -> f32 {
            (self.0 as u32 >> 8) as f32 / (1_u32 << 24) as f32
        }

        fn next_f64(&mut self) -> f64 {
            (self.0 >> 11) as f64 / (1_u64 << 53) as f64
        }

        fn next_bool(&mut self) -> bool {
            false
        }
    }

    /// Keeps the previous test rolls for a seeded `StdRng`.
    struct RandAdapter(StdRng);

    impl RandomAccess for RandAdapter {
        fn next_i32(&mut self) -> i32 {
            self.0.random()
        }

        fn next_i32_bounded(&mut self, bound: i32) -> i32 {
            self.0.random_range(0..bound)
        }

        fn next_i64(&mut self) -> i64 {
            self.0.random::<u64>() as i64
        }

        fn next_f32(&mut self) -> f32 {
            self.0.random()
        }

        fn next_f64(&mut self) -> f64 {
            self.0.random()
        }

        fn next_bool(&mut self) -> bool {
            self.0.random()
        }
    }

    fn warped_fungus() -> NetherFungusBlock {
        NetherFungusBlock::new(
            &vanilla_blocks::WARPED_FUNGUS,
            &vanilla_blocks::WARPED_NYLIUM,
            &vanilla_configured_features::WARPED_FUNGUS_PLANTED,
            BlockTag::SUPPORTS_WARPED_FUNGUS,
        )
    }

    fn crimson_fungus() -> NetherFungusBlock {
        NetherFungusBlock::new(
            &vanilla_blocks::CRIMSON_FUNGUS,
            &vanilla_blocks::CRIMSON_NYLIUM,
            &vanilla_configured_features::CRIMSON_FUNGUS_PLANTED,
            BlockTag::SUPPORTS_CRIMSON_FUNGUS,
        )
    }

    #[test]
    fn bonemeal_requires_matching_nylium_and_build_height() {
        init_vanilla_registry();
        let behavior = warped_fungus();
        let state = vanilla_blocks::WARPED_FUNGUS.default_state();
        let pos = BlockPos::ZERO;
        let matching = TestLevel::default()
            .with_block(pos.below(), vanilla_blocks::WARPED_NYLIUM.default_state());
        assert!(behavior.is_valid_bonemeal_target(state, &matching, pos));

        let wrong_nylium = TestLevel::default()
            .with_block(pos.below(), vanilla_blocks::CRIMSON_NYLIUM.default_state());
        assert!(!behavior.is_valid_bonemeal_target(state, &wrong_nylium, pos));

        let at_build_limit = TestLevel::default();
        let top = BlockPos::new(0, at_build_limit.max_y_exclusive() - 1, 0);
        let at_build_limit = TestLevel::default()
            .with_block(top.below(), vanilla_blocks::WARPED_NYLIUM.default_state());
        assert!(!behavior.is_valid_bonemeal_target(state, &at_build_limit, top));
    }

    #[test]
    fn bonemeal_uses_vanilla_success_probability() {
        init_vanilla_registry();
        let behavior = warped_fungus();
        let world_fixture = fresh_test_world("nether_fungus_probability");
        let world = &world_fixture.world;
        let state = vanilla_blocks::WARPED_FUNGUS.default_state();
        let pos = BlockPos::new(8, 64, 8);

        assert!(behavior.is_bonemeal_success(state, world, &mut FixedRng(0), pos));
        assert!(!behavior.is_bonemeal_success(state, world, &mut FixedRng(u64::MAX), pos));
    }

    #[test]
    fn bonemeal_grows_both_huge_fungus_variants() {
        init_vanilla_registry();
        init_behaviors();

        for (name, behavior, fungus, nylium, stem) in [
            (
                "warped_fungus_growth",
                warped_fungus(),
                &vanilla_blocks::WARPED_FUNGUS,
                &vanilla_blocks::WARPED_NYLIUM,
                &vanilla_blocks::WARPED_STEM,
            ),
            (
                "crimson_fungus_growth",
                crimson_fungus(),
                &vanilla_blocks::CRIMSON_FUNGUS,
                &vanilla_blocks::CRIMSON_NYLIUM,
                &vanilla_blocks::CRIMSON_STEM,
            ),
        ] {
            let world_fixture = fresh_test_world(name);
            let world = &world_fixture.world;
            let pos = BlockPos::new(8, 64, 8);
            insert_ready_full_chunk(world, ChunkPos::from_block_pos(pos));
            assert!(world.set_block(
                pos.below(),
                nylium.default_state(),
                UpdateFlags::UPDATE_NONE,
            ));
            let state = fungus.default_state();
            assert!(world.set_block(pos, state, UpdateFlags::UPDATE_NONE));

            behavior.perform_bonemeal(
                state,
                world,
                &mut RandAdapter(StdRng::seed_from_u64(1)),
                pos,
            );

            assert_eq!(world.get_block_state(pos).get_block(), stem);
            assert_eq!(world.get_block_state(pos.above_n(3)).get_block(), stem);
        }
    }
}
