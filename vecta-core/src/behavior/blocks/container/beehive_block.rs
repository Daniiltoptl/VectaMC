//! Beehive block behavior implementation.

use std::sync::{Arc, Weak};

use vecta_macros::block_behavior;
use vecta_registry::blocks::BlockRef;
use vecta_registry::blocks::block_state_ext::BlockStateExt;
use vecta_registry::blocks::properties::{
    BlockStateProperties, Direction, EnumProperty, IntProperty,
};
use vecta_registry::vanilla_block_entity_types;
use vecta_utils::types::UpdateFlags;
use vecta_utils::{BlockPos, BlockStateId};

use crate::behavior::block::{BlockBehavior, BlockEntityCreation};
use crate::behavior::context::BlockPlaceContext;
use crate::block_entity::BLOCK_ENTITIES;
use crate::world::World;

/// Behavior for beehive and bee nest blocks.
// TODO: Implement full vanilla beehive interactions, bee release, smoke/fire handling, loot/data components, and ticking.
#[block_behavior]
pub struct BeehiveBlock {
    block: BlockRef,
}

const HORIZONTAL_FACING: &EnumProperty<Direction> = &BlockStateProperties::HORIZONTAL_FACING;
const LEVEL_HONEY: &IntProperty = &BlockStateProperties::LEVEL_HONEY;

impl BeehiveBlock {
    /// Creates a new beehive block behavior.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }

    /// Vanilla `BeehiveBlock.resetHoneyLevel`.
    ///
    /// Vanilla reaches this through `releaseBeesAndResetHoneyLevel`, which also
    /// empties the hive into `Bee` entities. Vecta has no bee entity yet, so the
    /// stored occupants stay where they are.
    pub(crate) fn reset_honey_level(world: &Arc<World>, state: BlockStateId, pos: BlockPos) {
        world.set_block(
            pos,
            state.set_value(LEVEL_HONEY, 0),
            UpdateFlags::UPDATE_ALL,
        );
    }
}

impl BlockBehavior for BeehiveBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        Some(
            self.block
                .default_state()
                .set_value(HORIZONTAL_FACING, context.horizontal_direction().opposite()),
        )
    }

    fn new_block_entity(
        &self,
        level: Weak<World>,
        pos: BlockPos,
        state: BlockStateId,
    ) -> BlockEntityCreation {
        BlockEntityCreation::from_registered_factory(BLOCK_ENTITIES.create(
            &vanilla_block_entity_types::BEEHIVE,
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
        _world: &World,
        _pos: BlockPos,
        _direction: Direction,
    ) -> i32 {
        state.get_value(LEVEL_HONEY).into()
    }
}
