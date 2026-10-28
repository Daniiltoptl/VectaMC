//! Vanilla walk path-type classification.

mod collision;
pub mod node_evaluator;
mod path_evaluator;
mod settings;

pub use collision::WalkNodeCollision;
pub use node_evaluator::WalkNodeEvaluator;
pub use path_evaluator::WalkPathEvaluator;
use path_evaluator::does_block_have_partial_collision;
pub use settings::MobPathSettings;

use vecta_math::fast_floor;
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::blocks::properties::BlockStateProperties;
use vecta_registry::fluid::FluidState;
use vecta_registry::vanilla_block_tags::BlockTag;
use vecta_registry::vanilla_blocks;
use vecta_utils::{BlockPos, Direction, WorldAabb, axis::Axis};

use crate::behavior::{BLOCK_BEHAVIORS, BlockCollisionContext, BlockStateBehaviorExt as _};
use crate::entity::Mob;
use crate::entity::ai::node::{Node, NodeStore};
use crate::entity::ai::path::{
    PathComputationType, PathType, PathTypeSet, PathfindingContext, PathfindingMalus,
};
use crate::fluid::FluidStateExt as _;
use crate::world::LevelReader;

#[cfg(test)]
mod tests;
