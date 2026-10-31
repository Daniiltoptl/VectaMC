//! Viewer counting shared by container block entities.
//!
//! Mirrors Vanilla `ContainerOpenersCounter`: the counter is authoritative only
//! for the current tick. While at least one viewer is recorded, a scheduled
//! block tick recounts the real viewers within interaction range, so a
//! disconnect, a teleport or a menu replaced by another one can never leave a
//! container permanently "open".

use std::sync::Arc;

use glam::DVec3;
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::vanilla_game_events;
use vecta_utils::{BlockPos, BlockStateId, Downcast as _, WorldAabb, locks::SyncMutex};

use crate::entity::Entity;
use crate::player::Player;
use crate::world::World;
use crate::world::game_event::GameEventContext;

/// Vanilla `ContainerOpenersCounter.CHECK_TICK_DELAY`.
pub const CHECK_TICK_DELAY: i32 = 5;

/// Vanilla `ContainerOpenersCounter.getEntitiesWithContainerOpen` search slack.
const SEARCH_RANGE_SLACK: f64 = 4.0;

/// Block-entity reactions to its viewer count changing.
pub trait ContainerOpenersListener {
    /// Called when the first viewer opens the container.
    fn on_open(&self, world: &Arc<World>, pos: BlockPos, state: BlockStateId);

    /// Called when the last viewer closes the container.
    fn on_close(&self, world: &Arc<World>, pos: BlockPos, state: BlockStateId);

    /// Called after every increment, decrement and recount.
    fn opener_count_changed(
        &self,
        world: &Arc<World>,
        pos: BlockPos,
        state: BlockStateId,
        previous: i32,
        current: i32,
    );

    /// Vanilla `ContainerOpenersCounter.isOwnContainer`: whether the player's
    /// open menu is backed by this very container.
    ///
    /// This is the entity-side half of Vanilla's `ContainerUser` interface: a
    /// player counts as a viewer only while their open menu still owns this
    /// container and is still valid.
    fn is_own_container(&self, player: &Player) -> bool;
}

/// Number of entities that currently have a container open.
#[derive(Default)]
pub struct ContainerOpenersCounter {
    open_count: SyncMutex<i32>,
    max_interaction_range: SyncMutex<f64>,
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
        max_interaction_range: f64,
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
            Self::schedule_recheck(world, pos, state);
        }
        listener.opener_count_changed(world, pos, state, previous, previous + 1);

        let mut range = self.max_interaction_range.lock();
        *range = range.max(max_interaction_range);
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
            *self.max_interaction_range.lock() = 0.0;
        }
        listener.opener_count_changed(world, pos, state, previous, current);
    }

    /// Vanilla `ContainerOpenersCounter.recheckOpeners`: recounts the real
    /// viewers and resynchronises the counter with them.
    pub fn recheck_openers(
        &self,
        listener: &dyn ContainerOpenersListener,
        world: &Arc<World>,
        pos: BlockPos,
        state: BlockStateId,
    ) {
        let (open_count, max_range) = self.count_container_users(listener, world, pos);

        *self.max_interaction_range.lock() = max_range;

        let previous = {
            let mut count = self.open_count.lock();
            let previous = *count;
            *count = open_count;
            previous
        };

        if previous != open_count {
            let is_open = open_count != 0;
            if is_open && previous == 0 {
                listener.on_open(world, pos, state);
                world.game_event(
                    &vanilla_game_events::CONTAINER_OPEN,
                    pos,
                    &GameEventContext::new(None, None),
                );
            } else if !is_open {
                listener.on_close(world, pos, state);
                world.game_event(
                    &vanilla_game_events::CONTAINER_CLOSE,
                    pos,
                    &GameEventContext::new(None, None),
                );
            }
        }

        listener.opener_count_changed(world, pos, state, previous, open_count);
        if open_count > 0 {
            Self::schedule_recheck(world, pos, state);
        }
    }

    /// Vanilla `ContainerOpenersCounter.getEntitiesWithContainerOpen`, reduced
    /// to the two values the recount needs: how many viewers there are and the
    /// widest interaction range among them.
    fn count_container_users(
        &self,
        listener: &dyn ContainerOpenersListener,
        world: &Arc<World>,
        pos: BlockPos,
    ) -> (i32, f64) {
        let range = *self.max_interaction_range.lock() + SEARCH_RANGE_SLACK;
        let center = DVec3::new(
            f64::from(pos.x()) + 0.5,
            f64::from(pos.y()) + 0.5,
            f64::from(pos.z()) + 0.5,
        );
        let search_box = WorldAabb::new(
            center.x - range,
            center.y - range,
            center.z - range,
            center.x + range,
            center.y + range,
            center.z + range,
        );

        let entities = world.get_entities_in_aabb(&search_box);
        let mut count = 0;
        let mut max_range: f64 = 0.0;
        for entity in &entities {
            let Some(player) = entity.downcast_ref::<Player>() else {
                continue;
            };
            if player.is_spectator() || !listener.is_own_container(player) {
                continue;
            }
            count += 1;
            max_range = max_range.max(player.block_interaction_range());
        }
        (count, max_range)
    }

    /// Vanilla `ContainerOpenersCounter.scheduleRecheck`.
    fn schedule_recheck(world: &Arc<World>, pos: BlockPos, state: BlockStateId) {
        world.schedule_block_tick_default(pos, state.get_block(), CHECK_TICK_DELAY);
    }

    /// Returns the current viewer count.
    #[must_use]
    pub fn opener_count(&self) -> i32 {
        *self.open_count.lock()
    }

    /// Returns the widest interaction range seen among the recorded viewers.
    #[must_use]
    pub fn max_interaction_range(&self) -> f64 {
        *self.max_interaction_range.lock()
    }
}
