//! Vanilla `Shearable`: an entity that gives up drops when sheared.

use std::sync::Arc;

use vecta_protocol::packets::game::SoundSource;
use vecta_registry::item_stack::ItemStack;

use crate::entity::Entity;
use crate::world::World;

/// Vanilla `Shearable`.
pub trait Shearable: Entity {
    /// Vanilla `Shearable.readyForShearing`.
    fn ready_for_shearing(&self) -> bool;

    /// Vanilla `Shearable.shear`: play the shear sound, drop the loot and enter
    /// the sheared state.
    fn shear(&self, world: &Arc<World>, sound_source: SoundSource, tool: &ItemStack);
}
