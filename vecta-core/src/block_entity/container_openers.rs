//! Viewer counting shared by container block entities.
//!
//! Mirrors Vanilla `ContainerOpenersCounter` without its scheduled recheck,
//! which needs container users on the entity side (W04-023).

use std::sync::Arc;

use vecta_registry::vanilla_game_events;
use vecta_utils::{BlockPos, BlockStateId, locks::SyncMutex};

use crate::entity::Entity;
use crate::world::World;
use crate::world::game_event::GameEventContext;

/// Block-entity reactions to its viewer count changing.
pub trait ContainerOpenersListener {
    /// Called when the first viewer opens the container.
    fn on_open(&self, world: &Arc<World>, pos: BlockPos, state: BlockStateId);

    /// Called when the last viewer closes the container.
    fn on_close(&self, world: &Arc<World>, pos: BlockPos, state: BlockStateId);

    /// Called after every increment and decrement.
    fn opener_count_changed(
        &self,
        world: &Arc<World>,
        pos: BlockPos,
        state: BlockStateId,
        previous: i32,
        current: i32,
    );
}

/// Number of entities that currently have a container open.
#[derive(Default)]
pub struct ContainerOpenersCounter {
    open_count: SyncMutex<i32>,
}

impl ContainerOpenersCounter {
    /// Records one more viewer.
    pub fn increment_openers(
        &self,
        listener: &dyn ContainerOpenersListener,
        entity: &dyn Entity,
        world: &Arc<World>,
        pos: BlockPos,
        state: BlockStateId,
    ) {
        let previous = {
            let mut count = self.open_count.lock();
            let previous = *count;
            *count += 1;
            previous
        };
        if previous == 0 {
            listener.on_open(world, pos, state);
            world.game_event(
                &vanilla_game_events::CONTAINER_OPEN,
                pos,
                &GameEventContext::new(Some(entity), None),
            );
        }
        listener.opener_count_changed(world, pos, state, previous, previous + 1);
    }

    /// Records one viewer leaving.
    pub fn decrement_openers(
        &self,
        listener: &dyn ContainerOpenersListener,
        entity: &dyn Entity,
        world: &Arc<World>,
        pos: BlockPos,
        state: BlockStateId,
    ) {
        let previous = {
            let mut count = self.open_count.lock();
            let previous = *count;
            *count -= 1;
            previous
        };
        let current = previous - 1;
        if current == 0 {
            listener.on_close(world, pos, state);
            world.game_event(
                &vanilla_game_events::CONTAINER_CLOSE,
                pos,
                &GameEventContext::new(Some(entity), None),
            );
        }
        listener.opener_count_changed(world, pos, state, previous, current);
    }

    /// Returns the current viewer count.
    #[must_use]
    pub fn opener_count(&self) -> i32 {
        *self.open_count.lock()
    }
}
