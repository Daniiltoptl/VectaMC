//! Chest block entity implementation.
//!
//! A chest stores 27 slots and tracks how many players have its menu open so
//! clients can animate the lid through block events.

use std::{
    mem,
    sync::{
        Arc, Weak,
        atomic::{AtomicI32, Ordering},
    },
};

use simdnbt::ToNbtTag;
use simdnbt::borrow::{BaseNbtCompound as BorrowedNbtCompound, NbtCompound as NbtCompoundView};
use simdnbt::owned::{NbtCompound, NbtList, NbtTag};
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::vanilla_block_entity_types;
use vecta_utils::{BlockPos, BlockStateId, DowncastType, DowncastTypeKey, locks::SyncMutex};

use crate::block_entity::{BlockEntity, BlockEntityBase};
use crate::inventory::container::Container;
use crate::inventory::lock::{ContainerRef, SharedContainer};
use crate::world::World;

/// Number of slots in a chest (3 rows of 9).
pub const CHEST_SLOTS: usize = 27;

/// Block event id clients use to drive the chest lid.
pub const CHEST_LID_EVENT: i32 = 1;

/// Chest block entity.
pub struct ChestBlockEntity {
    base: Arc<BlockEntityBase>,
    container: Arc<SyncMutex<ChestContainer>>,
    container_ref: ContainerRef,
    /// Number of players currently looking at this chest's menu.
    open_count: AtomicI32,
}

struct ChestContainer {
    items: Vec<ItemStack>,
}

// SAFETY: This key is owned by Vecta and uniquely identifies `ChestBlockEntity`.
unsafe impl DowncastType for ChestBlockEntity {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:block_entity/chest");
}

// SAFETY: This key is owned by Vecta and uniquely identifies the independently
// lockable inventory data used by a chest block entity.
unsafe impl DowncastType for ChestContainer {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:container/chest");
}

impl ChestBlockEntity {
    /// Creates a new chest block entity.
    #[must_use]
    pub fn new(level: Weak<World>, pos: BlockPos, state: BlockStateId) -> Self {
        let base = Arc::new(BlockEntityBase::new(
            &vanilla_block_entity_types::CHEST,
            level,
            pos,
            state,
        ));
        let container = Arc::new(SyncMutex::new(ChestContainer {
            items: vec![ItemStack::empty(); CHEST_SLOTS],
        }));
        let shared_container: SharedContainer = Arc::<SyncMutex<ChestContainer>>::clone(&container);
        Self {
            container_ref: ContainerRef::owned_by_block_entity(shared_container, Arc::clone(&base)),
            base,
            container,
            open_count: AtomicI32::new(0),
        }
    }

    /// Returns how many players currently have this chest open.
    #[must_use]
    pub fn open_count(&self) -> i32 {
        self.open_count.load(Ordering::Relaxed)
    }

    /// Records that one more player opened this chest.
    ///
    /// Mirrors `ContainerOpenersCounter.incrementOpeners` for the part the
    /// server observes: the new viewer count is broadcast as a block event so
    /// clients open the lid.
    pub fn player_open(&self) {
        let count = self.open_count.fetch_add(1, Ordering::Relaxed) + 1;
        self.broadcast_lid_event(count);
    }

    /// Records that one player closed this chest.
    pub fn player_close(&self) {
        let previous = self.open_count.load(Ordering::Relaxed);
        let count = previous.saturating_sub(1).max(0);
        self.open_count.store(count, Ordering::Relaxed);
        if previous != count {
            self.broadcast_lid_event(count);
        }
    }

    fn broadcast_lid_event(&self, count: i32) {
        let Some(world) = self.get_level() else {
            return;
        };
        world.block_event(
            self.get_block_pos(),
            self.get_block_state().get_block(),
            CHEST_LID_EVENT,
            count,
        );
    }
}

impl BlockEntity for ChestBlockEntity {
    fn base(&self) -> &BlockEntityBase {
        &self.base
    }

    fn pre_remove_side_effects(&self, pos: BlockPos, _state: BlockStateId) {
        self.open_count.store(0, Ordering::Relaxed);
        let items = {
            let mut container = self.container.lock();
            mem::replace(&mut container.items, vec![ItemStack::empty(); CHEST_SLOTS])
        };
        let Some(world) = self.get_level() else {
            return;
        };
        for item in items {
            world.drop_item_stack(pos, item);
        }
    }

    fn load_additional(&self, nbt: &BorrowedNbtCompound<'_>) {
        let nbt_view: NbtCompoundView<'_, '_> = nbt.into();
        let mut container = self.container.lock();
        container.items.fill(ItemStack::empty());

        if let Some(items_list) = nbt_view.list("Items")
            && let Some(compounds) = items_list.compounds()
        {
            for compound in compounds {
                if let Some(slot) = compound.byte("Slot") {
                    let slot = slot as usize;
                    if slot < CHEST_SLOTS
                        && let Some(item) = ItemStack::from_borrowed_compound(&compound)
                    {
                        container.items[slot] = item;
                    }
                }
            }
        }
    }

    fn save_additional(&self, nbt: &mut NbtCompound) {
        let container = self.container.lock();
        let mut items: Vec<NbtCompound> = Vec::new();
        for (slot, item) in container.items.iter().enumerate() {
            if !item.is_empty()
                && let NbtTag::Compound(mut item_nbt) = item.clone().to_nbt_tag()
            {
                item_nbt.insert("Slot", slot as i8);
                items.push(item_nbt);
            }
        }
        nbt.insert("Items", NbtList::Compound(items));
    }

    fn trigger_event(&self, param_a: i32, param_b: i32) -> bool {
        if param_a == CHEST_LID_EVENT {
            // The lid animation itself is client-side; the server only owns the
            // viewer count that the event carries.
            let count = param_b.max(0);
            self.open_count.store(count, Ordering::Relaxed);
            return true;
        }
        false
    }

    fn container_ref(&self) -> Option<ContainerRef> {
        Some(self.container_ref.clone())
    }
}

impl Container for ChestContainer {
    fn items(&self) -> &[ItemStack] {
        &self.items
    }

    fn items_mut(&mut self) -> &mut [ItemStack] {
        &mut self.items
    }

    fn get_container_size(&self) -> usize {
        CHEST_SLOTS
    }

    fn set_item(&mut self, slot: usize, mut stack: ItemStack) {
        if slot < CHEST_SLOTS {
            let max_stack_size = self.get_max_stack_size_for_item(&stack);
            if !stack.is_empty() && stack.count() > max_stack_size {
                stack.set_count(max_stack_size);
            }
            self.items[slot] = stack;
        }
    }

    fn get_max_stack_size(&self) -> i32 {
        64
    }

    fn set_changed(&mut self) {}
}

#[cfg(test)]
mod tests {
    use vecta_registry::{init_vanilla_registry, vanilla_blocks, vanilla_items};

    use super::*;

    fn test_chest() -> ChestBlockEntity {
        init_vanilla_registry();
        ChestBlockEntity::new(
            Weak::new(),
            BlockPos::new(1, 2, 3),
            vanilla_blocks::CHEST.default_state(),
        )
    }

    #[test]
    fn open_count_tracks_viewers_without_going_negative() {
        let chest = test_chest();
        chest.player_open();
        chest.player_open();
        assert_eq!(chest.open_count(), 2);

        chest.player_close();
        assert_eq!(chest.open_count(), 1);
        chest.player_close();
        chest.player_close();
        assert_eq!(chest.open_count(), 0);
    }

    #[test]
    fn pre_remove_clears_items_and_viewers() {
        let chest = test_chest();
        chest
            .container
            .lock()
            .set_item(0, ItemStack::new(&vanilla_items::STONE));
        chest.player_open();

        chest.pre_remove_side_effects(
            BlockPos::new(1, 2, 3),
            vanilla_blocks::CHEST.default_state(),
        );

        assert_eq!(chest.open_count(), 0);
        let container = chest.container.lock();
        assert_eq!(container.items.len(), CHEST_SLOTS);
        assert!(container.items.iter().all(ItemStack::is_empty));
    }
}
