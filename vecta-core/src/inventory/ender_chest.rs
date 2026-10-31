//! Ender chest container implementation.

use std::ptr;
use std::sync::{Arc, Weak};

use vecta_registry::item_stack::ItemStack;
use vecta_utils::locks::SyncMutex;
use vecta_utils::{Downcast, DowncastType, DowncastTypeKey};

use crate::block_entity::entities::EnderChestBlockEntity;
use crate::block_entity::{BlockEntity, SharedBlockEntity};
use crate::inventory::container::Container;
use crate::player::Player;

/// Number of slots in an ender chest (3 rows of 9).
pub const ENDER_CHEST_SLOTS: usize = 27;

type WeakBlockEntity = Weak<dyn BlockEntity>;

/// Thread-safe reference to a player's ender chest container.
pub type SyncPlayerEnderChest = Arc<SyncMutex<PlayerEnderChestContainer>>;

/// The player's ender chest inventory.
pub struct PlayerEnderChestContainer {
    items: Vec<ItemStack>,
    active_chest: Option<WeakBlockEntity>,
}

// SAFETY: This key is owned by Vecta and uniquely identifies `PlayerEnderChestContainer`.
unsafe impl DowncastType for PlayerEnderChestContainer {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:inventory/player_ender_chest");
}

impl Default for PlayerEnderChestContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerEnderChestContainer {
    /// Creates a new, empty ender chest inventory.
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: vec![ItemStack::empty(); ENDER_CHEST_SLOTS],
            active_chest: None,
        }
    }

    /// Sets the block entity this container was most recently opened from.
    pub fn set_active_chest(&mut self, active_chest: WeakBlockEntity) {
        self.active_chest = Some(active_chest);
    }

    /// Clears the active block entity.
    pub fn clear_active_chest(&mut self) {
        self.active_chest = None;
    }

    /// Vanilla `PlayerEnderChestContainer.isActiveChest`: whether `chest` is the
    /// very block entity this container was opened from. The comparison is by
    /// identity, so a chest in another dimension at the same position does not
    /// count.
    #[must_use]
    pub fn is_active_chest(&self, chest: &dyn BlockEntity) -> bool {
        // `ptr::eq` compares the address and the metadata of a wide pointer, so
        // this is the same identity check as `activeChest == this` in Vanilla.
        self.active_chest()
            .is_some_and(|active| ptr::eq(Arc::as_ptr(&active), chest))
    }

    /// Vanilla `PlayerEnderChestContainer.startOpen`: the player's storage is
    /// shared, so the viewers are counted by the block entity it is open from.
    pub fn start_open(&self, player: &Player) {
        let Some(chest) = self.active_chest() else {
            return;
        };
        if let Some(ender_chest) = chest.downcast_ref::<EnderChestBlockEntity>() {
            ender_chest.start_open(player);
        }
    }

    /// Vanilla `PlayerEnderChestContainer.stopOpen`: forwards the close to the
    /// active chest and forgets it, so a later validity check can't consult a
    /// stale one.
    pub fn stop_open(&mut self, player: &Player) {
        if let Some(chest) = self.active_chest()
            && let Some(ender_chest) = chest.downcast_ref::<EnderChestBlockEntity>()
        {
            ender_chest.stop_open(player);
        }
        self.active_chest = None;
    }

    fn active_chest(&self) -> Option<SharedBlockEntity> {
        self.active_chest.as_ref()?.upgrade()
    }

    /// Checks if the container is still valid for the given player.
    #[must_use]
    pub fn still_valid(&self, player: &Player) -> bool {
        let Some(weak_chest) = &self.active_chest else {
            return true;
        };
        // A dropped weak handle means the chest was destroyed while open.
        let Some(chest) = weak_chest.upgrade() else {
            return false;
        };
        chest.base().is_valid_container_for(player)
    }
}

impl Container for PlayerEnderChestContainer {
    fn items(&self) -> &[ItemStack] {
        &self.items
    }

    fn items_mut(&mut self) -> &mut [ItemStack] {
        &mut self.items
    }

    fn set_changed(&mut self) {
        // Player data saving handles change tracking for this inventory.
    }
}
