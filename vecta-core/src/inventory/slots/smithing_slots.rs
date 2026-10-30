//! Smithing table result slot handler.
use std::sync::Arc;

use vecta_registry::item_stack::ItemStack;
use vecta_registry::level_events;
use vecta_utils::{BlockPos, locks::Shared};

use crate::{
    inventory::{
        container::{ResultContainer, SimpleContainer},
        prelude::*,
    },
    world::World,
};

/// Number of smithing inputs, all of which are consumed by one take.
const SMITHING_INPUT_SLOTS: usize = 3;

/// Result slot handler for a smithing table.
///
/// The menu owns recipe matching and the result stack; this handler consumes one
/// item from each of the three inputs and plays vanilla's
/// `levelEvent(1044)` "smithing table used" sound.
pub struct SmithingResultHandler {
    input_container: Shared<SimpleContainer>,
    result_container: Shared<ResultContainer>,
    block_pos: BlockPos,
    world: Arc<World>,
}

impl SmithingResultHandler {
    /// Creates a new handler.
    pub const fn new(
        input_container: Shared<SimpleContainer>,
        result_container: Shared<ResultContainer>,
        block_pos: BlockPos,
        world: Arc<World>,
    ) -> Self {
        Self {
            input_container,
            result_container,
            block_pos,
            world,
        }
    }
}

impl ResultHandler for SmithingResultHandler {
    fn result_container(&self) -> ContainerRef {
        ContainerRef::from(Arc::clone(&self.result_container))
    }

    fn dependencies(&self) -> Vec<ContainerRef> {
        vec![ContainerRef::from(Arc::clone(&self.input_container))]
    }

    fn update_result(&self, _guard: &mut ContainerLockGuard) {}

    fn on_result_taken(
        &self,
        guard: &mut ContainerLockGuard,
        _player: &Player,
    ) -> Option<ItemStack> {
        let Some(inputs) = guard.get_mut(ContainerId::from_arc(&self.input_container)) else {
            log::warn!("input container not locked while taking a smithing result");
            return None;
        };
        // `SmithingMenu.shrinkStackInSlot` for slots 0, 1 and 2.
        for slot in 0..SMITHING_INPUT_SLOTS {
            let stack = inputs.get_item_mut(slot);
            if !stack.is_empty() {
                stack.shrink(1);
            }
        }
        inputs.set_changed();

        self.world.level_event(
            level_events::SOUND_SMITHING_TABLE_USED,
            self.block_pos,
            0,
            None,
        );

        None
    }

    fn is_result_valid(&self, _guard: &ContainerLockGuard, _player: &Player) -> bool {
        true
    }
}
