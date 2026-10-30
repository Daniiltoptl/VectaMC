use std::sync::{
    Arc,
    atomic::{AtomicI32, Ordering},
};

use vecta_registry::{item_stack::ItemStack, stat::vanilla_stat_types};
use vecta_utils::{DowncastType, DowncastTypeKey};

use crate::{
    inventory::{
        lock::{ContainerLockGuard, ContainerRef},
        slots::{ResultHandler, Slot, SlotStorage},
    },
    player::Player,
};

/// How a result slot accounts the crafted-item statistic on take.
///
/// Vanilla 26.3 has three different result-slot contracts: the crafting
/// `ResultSlot` accumulates the removed count across remove/quick-craft/
/// swap-craft and awards it once per take, the stonecutter and smithing menus
/// award the count of the stack passed to `onTake`, and the anvil, grindstone
/// and loom result slots award no crafted statistic at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CraftedStat {
    /// The slot never awards `ITEM_CRAFTED`.
    Ignored,
    /// Award the count accumulated by remove/quick-craft/swap-craft.
    Accumulated,
    /// Award the count of the stack handed to [`Slot::on_take`].
    TakenStack,
}

/// A fake Slot that contains the resulting item of f.e. a craft
pub struct ResultSlot {
    handler: Arc<dyn ResultHandler + Send + Sync>,
    storage: SlotStorage,
    crafted_stat: CraftedStat,
    /// Items removed from the preview since the last `on_take`, used by
    /// [`CraftedStat::Accumulated`].
    remove_count: AtomicI32,
}

// SAFETY: This key uniquely identifies Vecta's `ResultSlot`.
unsafe impl DowncastType for ResultSlot {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:slot/result");
}

impl ResultSlot {
    /// Creates a new `ResultSlot` backed by the handler's result container.
    pub fn new(handler: impl ResultHandler + 'static) -> Self {
        let result_container = handler.result_container();
        let storage =
            SlotStorage::physical(result_container, 0).with_dependencies(handler.dependencies());
        Self {
            handler: Arc::new(handler),
            storage,
            crafted_stat: CraftedStat::Ignored,
            remove_count: AtomicI32::new(0),
        }
    }

    /// Selects how this slot reports crafted items.
    #[must_use]
    pub const fn with_crafted_stat(mut self, crafted_stat: CraftedStat) -> Self {
        self.crafted_stat = crafted_stat;
        self
    }

    /// Awards the crafted statistic for one take, mirroring
    /// `ResultSlot.checkTakeAchievements`.
    fn check_take_achievements(&self, carried: &ItemStack, player: &Player) {
        let accumulated = self.remove_count.swap(0, Ordering::Relaxed);
        let count = match self.crafted_stat {
            CraftedStat::Ignored => return,
            CraftedStat::Accumulated => accumulated,
            CraftedStat::TakenStack => carried.count(),
        };
        if count > 0 {
            player.award_stat_with_count(&vanilla_stat_types::ITEM_CRAFTED, carried.item(), count);
        }
    }

    /// Returns the result container this slot derived from its handler.
    pub(crate) fn result_container(&self) -> &ContainerRef {
        let Some((container, _)) = self.storage.physical_backing() else {
            unreachable!("ResultSlot always has physical storage");
        };
        container
    }
}

impl Slot for ResultSlot {
    fn storage(&self) -> &SlotStorage {
        &self.storage
    }

    fn get_item<'a>(&self, guard: &'a ContainerLockGuard) -> &'a ItemStack {
        guard
            .get(self.result_container().container_id())
            .expect("failed to get item from result container")
            .get_item(0)
    }

    fn get_item_mut<'a>(&self, guard: &'a mut ContainerLockGuard) -> &'a mut ItemStack {
        guard
            .get_mut(self.result_container().container_id())
            .expect("failed to get item mutabily from result container")
            .get_item_mut(0)
    }

    fn set_item(&self, guard: &mut ContainerLockGuard, stack: ItemStack) {
        guard
            .get_mut(self.result_container().container_id())
            .expect("failed to get item mutabily from result container")
            .set_item(0, stack);
    }

    fn get_max_stack_size(&self, guard: &ContainerLockGuard) -> i32 {
        guard
            .get(self.result_container().container_id())
            .expect("result container not locked")
            .get_max_stack_size()
    }

    fn set_changed(&self, guard: &mut ContainerLockGuard) {
        guard
            .get_mut(self.result_container().container_id())
            .expect("result container not locked")
            .set_changed();
    }

    fn get_container_slot(&self) -> usize {
        0
    }

    fn on_take(
        &self,
        guard: &mut ContainerLockGuard,
        stack: &ItemStack,
        player: &Player,
    ) -> Option<ItemStack> {
        self.check_take_achievements(stack, player);
        self.handler.on_result_taken(guard, player)
    }

    fn on_quick_craft(&self, picked: &ItemStack, original: &ItemStack, player: &Player) {
        if self.crafted_stat != CraftedStat::Accumulated {
            return;
        }
        let count = original.count() - picked.count();
        if count > 0 {
            self.remove_count.fetch_add(count, Ordering::Relaxed);
            self.check_take_achievements(original, player);
        }
    }

    fn on_swap_craft(&self, count: i32) {
        if self.crafted_stat == CraftedStat::Accumulated && count > 0 {
            self.remove_count.fetch_add(count, Ordering::Relaxed);
        }
    }

    fn remove(&self, guard: &mut ContainerLockGuard, amount: i32) -> ItemStack {
        let count = self.get_item(guard).count();
        if count > 0 && amount > 0 {
            self.remove_count
                .fetch_add(amount.min(count), Ordering::Relaxed);
        }
        let item = self.get_item_mut(guard);
        if item.is_empty() || amount <= 0 {
            return ItemStack::empty();
        }
        item.split(amount)
    }

    fn is_fake(&self) -> bool {
        true
    }

    fn allow_modification(&self, _guard: &ContainerLockGuard, _player: &Player) -> bool {
        false
    }

    fn may_place(&self, _stack: &ItemStack) -> bool {
        false
    }

    fn may_pickup(&self, guard: &ContainerLockGuard, player: &Player) -> bool {
        self.handler.is_result_valid(guard, player)
    }
}

#[cfg(test)]
mod tests {
    use vecta_utils::locks::IntoShared;

    use super::*;
    use crate::inventory::container::SimpleContainer;

    struct NoopResultHandler(ContainerRef);

    impl ResultHandler for NoopResultHandler {
        fn result_container(&self) -> ContainerRef {
            self.0.clone()
        }

        fn dependencies(&self) -> Vec<ContainerRef> {
            Vec::new()
        }

        fn update_result(&self, _guard: &mut ContainerLockGuard) {}

        fn on_result_taken(
            &self,
            _guard: &mut ContainerLockGuard,
            _player: &Player,
        ) -> Option<ItemStack> {
            None
        }

        fn is_result_valid(&self, _guard: &ContainerLockGuard, _player: &Player) -> bool {
            true
        }
    }

    #[test]
    fn constructor_uses_the_handlers_result_container() {
        let container = ContainerRef::from(SimpleContainer::new(1).into_shared());
        let expected = container.container_id();

        let slot = ResultSlot::new(NoopResultHandler(container));

        assert_eq!(slot.storage().physical_key(), Some((expected, 0)));
    }
}
