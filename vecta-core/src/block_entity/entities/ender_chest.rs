//! Ender chest block entity implementation.

use std::sync::{Arc, Weak};

use glam::DVec3;
use simdnbt::borrow::BaseNbtCompound;
use simdnbt::owned::NbtCompound;
use vecta_protocol::packets::game::SoundSource;
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::{sound_events, vanilla_block_entity_types};
use vecta_utils::{BlockPos, BlockStateId, DowncastType, DowncastTypeKey};

use crate::block_entity::container_openers::{ContainerOpenersCounter, ContainerOpenersListener};
use crate::block_entity::{BlockEntity, BlockEntityBase, BlockEntityLifecycleExt as _};
use crate::entity::Entity as _;
use crate::player::Player;
use crate::world::World;

/// Block event id that drives the lid animation (`EVENT_SET_OPEN_COUNT`).
const EVENT_SET_OPEN_COUNT: i32 = 1;

/// Ender chest block entity.
pub struct EnderChestBlockEntity {
    base: Arc<BlockEntityBase>,
    openers: ContainerOpenersCounter,
}

// SAFETY: This key is owned by Vecta and uniquely identifies `EnderChestBlockEntity`.
unsafe impl DowncastType for EnderChestBlockEntity {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:block_entity/ender_chest");
}

impl EnderChestBlockEntity {
    /// Creates a new ender chest block entity.
    #[must_use]
    pub fn new(level: Weak<World>, pos: BlockPos, state: BlockStateId) -> Self {
        Self {
            base: Arc::new(BlockEntityBase::new(
                &vanilla_block_entity_types::ENDER_CHEST,
                level,
                pos,
                state,
            )),
            openers: ContainerOpenersCounter::default(),
        }
    }

    /// Returns how many players currently have this chest open.
    #[must_use]
    pub fn open_count(&self) -> i32 {
        self.openers.opener_count()
    }

    /// Records a player opening this chest's menu (`EnderChestBlockEntity.startOpen`).
    pub fn start_open(&self, player: &Player) {
        if self.is_removed() || player.is_spectator() {
            return;
        }
        let Some(world) = self.get_level() else {
            return;
        };
        self.openers.increment_openers(
            self,
            player,
            &world,
            self.get_block_pos(),
            self.get_block_state(),
            player.block_interaction_range(),
        );
    }

    /// Records a player closing this chest's menu (`EnderChestBlockEntity.stopOpen`).
    pub fn stop_open(&self, player: &Player) {
        if self.is_removed() || player.is_spectator() {
            return;
        }
        let Some(world) = self.get_level() else {
            return;
        };
        self.openers.decrement_openers(
            self,
            player,
            &world,
            self.get_block_pos(),
            self.get_block_state(),
        );
    }

    /// Vanilla `EnderChestBlockEntity.recheckOpen`, driven by `EnderChestBlock.tick`.
    pub fn recheck_open(&self) {
        if self.is_removed() {
            return;
        }
        let Some(world) = self.get_level() else {
            return;
        };
        self.openers
            .recheck_openers(self, &world, self.get_block_pos(), self.get_block_state());
    }

    /// Vanilla `EnderChestBlockEntity` open/close sound: played at the centre of
    /// the block, at a fixed volume of 0.5.
    fn play_lid_sound(world: &Arc<World>, pos: BlockPos, opening: bool) {
        let sound = if opening {
            &sound_events::BLOCK_ENDER_CHEST_OPEN
        } else {
            &sound_events::BLOCK_ENDER_CHEST_CLOSE
        };
        world.play_sound_at(
            sound,
            SoundSource::Blocks,
            DVec3::new(
                f64::from(pos.x()) + 0.5,
                f64::from(pos.y()) + 0.5,
                f64::from(pos.z()) + 0.5,
            ),
            0.5,
            rand::random::<f32>() * 0.1 + 0.9,
            None,
        );
    }
}

impl ContainerOpenersListener for EnderChestBlockEntity {
    fn on_open(&self, world: &Arc<World>, pos: BlockPos, _state: BlockStateId) {
        Self::play_lid_sound(world, pos, true);
    }

    fn on_close(&self, world: &Arc<World>, pos: BlockPos, _state: BlockStateId) {
        Self::play_lid_sound(world, pos, false);
    }

    fn opener_count_changed(
        &self,
        world: &Arc<World>,
        pos: BlockPos,
        state: BlockStateId,
        _previous: i32,
        current: i32,
    ) {
        // `openerCountChanged` broadcasts the new count so clients animate the lid.
        world.block_event(pos, state.get_block(), EVENT_SET_OPEN_COUNT, current);
    }

    fn is_own_container(&self, player: &Player) -> bool {
        // Vanilla `isOwnContainer`: the player's ender storage is open from this
        // very block entity. The storage is shared, so the container reference
        // cannot identify it.
        player.ender_chest_inventory.lock().is_active_chest(self)
    }
}

impl BlockEntity for EnderChestBlockEntity {
    fn base(&self) -> &BlockEntityBase {
        &self.base
    }

    fn trigger_event(&self, param_a: i32, _param_b: i32) -> bool {
        // The lid animates client-side; accepting the event lets the server
        // broadcast it.
        param_a == EVENT_SET_OPEN_COUNT
    }

    fn load_additional(&self, _nbt: &BaseNbtCompound<'_>) {}

    fn save_additional(&self, _nbt: &mut NbtCompound) {}
}
