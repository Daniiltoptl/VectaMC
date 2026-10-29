//! Loom result slot handler.
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use vecta_protocol::packets::game::SoundSource;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::sound_events;
use vecta_utils::{BlockPos, locks::Shared};

use crate::{
    inventory::{
        container::{ResultContainer, SimpleContainer},
        prelude::*,
    },
    world::World,
};

/// Result slot handler for a loom.
///
/// The menu owns pattern selection and the result stack; this handler consumes
/// one banner and one dye per take and plays vanilla's take sound at most once
/// per game tick.
pub struct LoomResultHandler {
    input_container: Shared<SimpleContainer>,
    result_container: Shared<ResultContainer>,
    block_pos: BlockPos,
    world: Arc<World>,
    last_sound_time: AtomicI64,
}

impl LoomResultHandler {
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
            last_sound_time: AtomicI64::new(0),
        }
    }
}

impl ResultHandler for LoomResultHandler {
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
            log::warn!("input container not locked while taking a loom result");
            return None;
        };
        inputs.remove_item(0, 1);
        inputs.remove_item(1, 1);

        let game_time = self.world.game_time();
        if self.last_sound_time.load(Ordering::Relaxed) != game_time {
            self.world.play_sound(
                &sound_events::UI_LOOM_TAKE_RESULT,
                SoundSource::Blocks,
                self.block_pos,
                1.0,
                1.0,
                None,
            );
            self.last_sound_time.store(game_time, Ordering::Relaxed);
        }

        None
    }

    fn is_result_valid(&self, _guard: &ContainerLockGuard, _player: &Player) -> bool {
        true
    }
}
