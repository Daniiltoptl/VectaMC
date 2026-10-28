//! Display and UI entity implementations.

use std::sync::Arc;

use vecta_registry::entity_type::EntityTypeRef;
use vecta_registry::vanilla_entities;
use vecta_utils::{BlockPos, Direction};

use crate::entity::{ENTITIES, SharedEntity, next_entity_id};
use crate::world::World;

mod block_display;
mod glow_item_frame;
mod item_frame;
mod leash_fence_knot;

pub use block_display::BlockDisplayEntity;
pub use glow_item_frame::GlowItemFrameEntity;
pub use item_frame::ItemFrameEntity;
pub use leash_fence_knot::LeashFenceKnotEntity;

/// Creates an item frame of the given variant attached to `block_pos`.
///
/// Mirrors `HangingEntityItem.useOn`, which constructs the concrete hanging
/// entity itself instead of going through the generic spawn path.
pub(crate) fn create_item_frame(
    entity_type: EntityTypeRef,
    block_pos: BlockPos,
    direction: Direction,
    world: &Arc<World>,
) -> Option<SharedEntity> {
    if !ENTITIES.has_factory(entity_type) {
        return None;
    }

    let id = next_entity_id();
    let weak_world = Arc::downgrade(world);
    if entity_type == &vanilla_entities::GLOW_ITEM_FRAME {
        let entity: SharedEntity = Arc::new(GlowItemFrameEntity::new_attached(
            entity_type,
            id,
            block_pos,
            direction,
            weak_world,
        ));
        Some(entity)
    } else if entity_type == &vanilla_entities::ITEM_FRAME {
        let entity: SharedEntity = Arc::new(ItemFrameEntity::new_attached(
            entity_type,
            id,
            block_pos,
            direction,
            weak_world,
        ));
        Some(entity)
    } else {
        None
    }
}
