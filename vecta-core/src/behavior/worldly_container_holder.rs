//! Vanilla `WorldlyContainerHolder`: blocks that expose a transient automation
//! container built from their block state instead of a block entity.

use std::sync::Arc;

use vecta_utils::{BlockPos, BlockStateId};

use crate::inventory::prelude::ContainerRef;
use crate::world::World;

/// Blocks whose automation view is derived from state, like the composter.
///
/// Vanilla `HopperBlockEntity.getBlockContainer` consults the holder of the
/// block **before** block entities, and a holder never falls through to one.
pub trait WorldlyContainerHolder: Send + Sync {
    /// Vanilla `WorldlyContainerHolder.getContainer`. The returned container is
    /// transient: it exists only for the current automation read and performs
    /// its world mutation from `set_changed`, like `ComposterBlock`'s
    /// `InputContainer`/`OutputContainer`.
    fn get_container(&self, state: BlockStateId, world: &Arc<World>, pos: BlockPos)
    -> ContainerRef;
}
