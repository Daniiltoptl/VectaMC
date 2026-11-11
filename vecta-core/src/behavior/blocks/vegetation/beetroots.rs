use std::sync::Arc;

use vecta_macros::block_behavior;
use vecta_registry::{
    blocks::{
        BlockRef,
        properties::{BlockStateProperties, IntProperty},
    },
    item_stack::ItemStack,
    vanilla_items,
};
use vecta_utils::BlockStateId;

use crate::{
    behavior::blocks::vegetation::{
        bonemealable::{Bonemealable, CropBonemealExt},
        crop_block::CropLike,
    },
    world::{LevelReader, World},
};
use vecta_utils::random::RandomAccess;

/// Behavior for the Beetroots Block
#[block_behavior]
pub struct BeetrootBlock {
    block: BlockRef,
}

const AGE: &IntProperty = &BlockStateProperties::AGE_3;

impl BeetrootBlock {
    /// Creates a new crop block behavior with a custom age property.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }
}

impl CropLike for BeetrootBlock {
    fn block(&self) -> BlockRef {
        self.block
    }

    fn age_property(&self) -> &IntProperty {
        AGE
    }

    fn max_age(&self) -> u8 {
        AGE.max
    }

    fn clone_item_stack(&self) -> ItemStack {
        ItemStack::new(&vanilla_items::BEETROOT_SEEDS)
    }

    fn should_random_tick(&self) -> bool {
        rand::random_range(0..3) != 0
    }
}

impl Bonemealable for BeetrootBlock {
    fn get_bonemeal_age_increase(&self, _world: &Arc<World>, rng: &mut dyn RandomAccess) -> u8 {
        (rng.next_i32_between(2, 5) / 3) as u8
    }

    fn is_valid_bonemeal_target(
        &self,
        state: BlockStateId,
        _world: &dyn LevelReader,
        _pos: vecta_utils::BlockPos,
    ) -> bool {
        !self.is_max_age(state)
    }

    fn perform_bonemeal(
        &self,
        state: BlockStateId,
        world: &Arc<World>,
        rng: &mut dyn RandomAccess,
        pos: vecta_utils::BlockPos,
    ) {
        self.default_perform_bonemeal(state, world, rng, pos);
    }
}
