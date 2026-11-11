use std::sync::LazyLock;

use vecta_registry::item_stack::ItemStack;
use vecta_registry::items::ItemRef;
use vecta_utils::types::InteractionHand;

use crate::inventory::{
    container::Container,
    equipment::{EntityEquipment, EquipmentSlot},
    recipe_book::StackedItemContents,
};

use super::core::PlayerInventory;

/// Maps vanilla player-container indices 36-42 to equipment slots.
const fn slot_to_equipment(slot: usize) -> Option<EquipmentSlot> {
    match slot {
        36 => Some(EquipmentSlot::Feet),
        37 => Some(EquipmentSlot::Legs),
        38 => Some(EquipmentSlot::Chest),
        39 => Some(EquipmentSlot::Head),
        40 => Some(EquipmentSlot::OffHand),
        41 => Some(EquipmentSlot::Body),
        42 => Some(EquipmentSlot::Saddle),
        _ => None,
    }
}

/// The equipment slot for an armor/offhand container index.
///
/// # Panics
/// Panics if `index` is not an equipment index. Menu sections restrict
/// themselves to [`PlayerInventory::ARMOR_TOP_DOWN`] and
/// [`PlayerInventory::SLOT_OFFHAND`], so this is unreachable from them.
pub(crate) const fn armor_equipment(index: usize) -> EquipmentSlot {
    slot_to_equipment(index).expect("armor sections only cover armor indices")
}

impl PlayerInventory {
    /// Vanilla `Inventory.isUsableForCrafting`: damaged, enchanted and renamed
    /// stacks are never consumed by recipe placement.
    #[must_use]
    pub fn is_usable_for_crafting(stack: &ItemStack) -> bool {
        !stack.is_damaged()
            && !stack.is_enchanted()
            && stack
                .get(vecta_registry::data_components::vanilla_components::CUSTOM_NAME)
                .is_none()
    }

    /// Vanilla `Inventory.fillStackedContents`.
    pub fn fill_stacked_contents(&self, contents: &mut StackedItemContents) {
        for item in &self.items {
            contents.account_simple_stack(item);
        }
    }

    /// Vanilla `Inventory.findSlotMatchingCraftingIngredient`.
    #[must_use]
    pub fn find_slot_matching_crafting_ingredient(
        &self,
        item: ItemRef,
        existing_item: &ItemStack,
    ) -> Option<usize> {
        self.items.iter().position(|candidate| {
            !candidate.is_empty()
                && candidate.item == item
                && Self::is_usable_for_crafting(candidate)
                && (existing_item.is_empty()
                    || ItemStack::is_same_item_same_components(existing_item, candidate))
        })
    }

    /// Vanilla `Inventory.getSlotWithRemainingSpace`: the selected slot and the
    /// offhand are preferred before the rest of the inventory.
    #[must_use]
    pub fn slot_with_remaining_space(&self, stack: &ItemStack) -> Option<usize> {
        for slot in [self.selected as usize, Self::SLOT_OFFHAND] {
            if Self::has_remaining_space_for_item(self.get_item(slot), stack) {
                return Some(slot);
            }
        }

        (0..self.get_container_size())
            .find(|slot| Self::has_remaining_space_for_item(self.get_item(*slot), stack))
    }

    /// Vanilla `Inventory.getAmountOfFreeSlotsInInventory`: empty main slots.
    #[must_use]
    pub fn amount_of_free_slots(&self) -> usize {
        self.items[..Self::INVENTORY_SIZE]
            .iter()
            .filter(|item| item.is_empty())
            .count()
    }

    /// Vanilla `Inventory.placeItemBackInInventory`: fills stacks with room and
    /// then empty main slots, leaving whatever does not fit in `stack` so the
    /// caller can drop it.
    ///
    /// Vanilla's slot search also visits equipment indices; Vecta keeps the
    /// main inventory as the destination so a returned grid item can never be
    /// lost to a failing equipment placement.
    pub fn place_item_back_in_inventory(&mut self, stack: &mut ItemStack) {
        while !stack.is_empty() {
            let Some(slot) = self
                .slot_with_remaining_space(stack)
                .filter(|slot| *slot < Self::INVENTORY_SIZE || *slot == Self::SLOT_OFFHAND)
                .or_else(|| self.get_free_slot().map(usize::from))
            else {
                return;
            };

            let space = self.get_max_stack_size_for_item(stack) - self.get_item(slot).count();
            let mut part = stack.split(space);
            let leftover = self.add_resource_to_slot(slot, &mut part);
            if leftover > 0 {
                stack.set_count(stack.count() + leftover);
                return;
            }
        }
    }

    fn has_remaining_space_for_item(slot_item: &ItemStack, new_item: &ItemStack) -> bool {
        !slot_item.is_empty()
            && ItemStack::is_same_item_same_components(slot_item, new_item)
            && slot_item.is_stackable()
            && slot_item.count() < slot_item.max_stack_size()
    }

    /// Vanilla `Inventory.addResource(slot, stack)`: returns how many items
    /// could not be added.
    fn add_resource_to_slot(&mut self, slot: usize, stack: &mut ItemStack) -> i32 {
        let count = stack.count();
        if self.get_item(slot).is_empty() {
            self.set_item(slot, stack.copy_with_count(0));
        }

        let space = self.get_max_stack_size_for_item(stack) - self.get_item(slot).count();
        let to_add = i32::min(count, space);
        if to_add == 0 {
            return count;
        }

        stack.shrink(to_add);
        self.get_item_mut(slot).grow(to_add);
        count - to_add
    }

    /// Applies vanilla `ItemUtils.createFilledResult` to a held item.
    ///
    /// Mutates the held stack and inventory, returning only the result stack that
    /// should be dropped by the caller. When `limit_creative_stack_size` is set,
    /// creative inventory insertion discards leftover result items instead of
    /// dropping them.
    pub fn apply_filled_result(
        &mut self,
        hand: InteractionHand,
        mut result_stack: ItemStack,
        has_infinite_materials: bool,
        limit_creative_stack_size: bool,
    ) -> ItemStack {
        if limit_creative_stack_size && has_infinite_materials {
            if !self.contains_stack(&result_stack) {
                let _ = self.add(&mut result_stack);
            }
            return ItemStack::empty();
        }

        if !has_infinite_materials {
            self.shrink_item_in_hand(hand, 1);
        }

        if self.get_item_in_hand(hand).is_empty() {
            self.set_item_in_hand(hand, result_stack);
            return ItemStack::empty();
        }

        let added = self.add(&mut result_stack);
        if added {
            ItemStack::empty()
        } else {
            result_stack
        }
    }

    /// Tries to add `stack` to the main inventory, returning whatever did
    /// not fit (empty if it was fully absorbed).
    pub fn add_or_return(&mut self, mut stack: ItemStack) -> ItemStack {
        if self.add(&mut stack) {
            ItemStack::empty()
        } else {
            stack
        }
    }
}

/// Static empty item stack for returning references to invalid slots.
static EMPTY_ITEM: LazyLock<ItemStack> = LazyLock::new(ItemStack::empty);

/// Error returned when a carried-item packet selects a non-hotbar slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidHotbarSlot;

impl Container for PlayerInventory {
    fn items(&self) -> &[ItemStack] {
        &self.items
    }

    fn items_mut(&mut self) -> &mut [ItemStack] {
        &mut self.items
    }

    fn get_container_size(&self) -> usize {
        Self::CONTAINER_SIZE
    }

    /// Adds an item to the player's main inventory (slots 0-35 only).
    ///
    /// Overrides the default `Container::add()` to prevent items from being
    /// placed in armor or equipment slots. Matches vanilla's `Inventory.add()`
    /// behavior which only adds to `this.items` (the 36 main slots).
    fn add(&mut self, stack: &mut ItemStack) -> bool {
        if stack.is_empty() {
            return true;
        }

        let max_size = self.get_max_stack_size_for_item(stack);
        let mut changed = false;

        // Vanilla prioritizes the selected slot, then an existing compatible
        // offhand stack, before scanning the remaining main inventory.
        if stack.is_stackable() {
            let selected = self.selected as usize;
            for slot in [selected, Self::SLOT_OFFHAND] {
                if stack.is_empty() {
                    if changed {
                        self.set_changed();
                    }
                    return true;
                }
                let existing = &mut self.items[slot];
                if !existing.is_empty() && ItemStack::is_same_item_same_components(existing, stack)
                {
                    let space = max_size - existing.count();
                    if space > 0 {
                        let to_add = stack.count().min(space);
                        existing.grow(to_add);
                        stack.shrink(to_add);
                        changed = true;
                    }
                }
            }

            for slot in 0..Self::INVENTORY_SIZE {
                if stack.is_empty() {
                    if changed {
                        self.set_changed();
                    }
                    return true;
                }
                if slot == selected {
                    continue;
                }
                let existing = &mut self.items[slot];
                if !existing.is_empty() && ItemStack::is_same_item_same_components(existing, stack)
                {
                    let space = max_size - existing.count();
                    if space > 0 {
                        let to_add = stack.count().min(space);
                        existing.grow(to_add);
                        stack.shrink(to_add);
                        changed = true;
                    }
                }
            }
        }

        // Second pass: try empty slots in main inventory only
        for slot in 0..Self::INVENTORY_SIZE {
            if stack.is_empty() {
                if changed {
                    self.set_changed();
                }
                return true;
            }
            if self.items[slot].is_empty() {
                let to_place = stack.count().min(max_size);
                self.items[slot] = stack.split(to_place);
                changed = true;
            }
        }

        if changed {
            self.set_changed();
        }
        stack.is_empty()
    }

    fn get_item(&self, slot: usize) -> &ItemStack {
        if slot < Self::CONTAINER_SIZE {
            &self.items[slot]
        } else {
            &EMPTY_ITEM
        }
    }

    fn get_item_mut(&mut self, slot: usize) -> &mut ItemStack {
        assert!(slot < Self::CONTAINER_SIZE, "Invalid slot index: {slot}");
        &mut self.items[slot]
    }

    fn set_item(&mut self, slot: usize, stack: ItemStack) {
        if slot == self.selected as usize {
            let _ = EntityEquipment::set(self, EquipmentSlot::MainHand, stack);
            return;
        }
        if let Some(equipment_slot) = slot_to_equipment(slot) {
            let _ = EntityEquipment::set(self, equipment_slot, stack);
            return;
        }
        if slot < Self::INVENTORY_SIZE {
            self.items[slot] = stack;
        }
        self.set_changed();
    }

    fn is_empty(&self) -> bool {
        self.items.iter().all(ItemStack::is_empty)
    }

    fn set_changed(&mut self) {
        self.times_changed = self.times_changed.wrapping_add(1);
    }

    fn clear_content(&mut self) -> i32 {
        let mut count = 0;
        for item in &mut self.items {
            count += item.count();
            *item = ItemStack::empty();
        }
        if count > 0 {
            self.set_changed();
        }
        count
    }

    fn clear_content_matching(&mut self, predicate: &mut dyn FnMut(&mut ItemStack) -> bool) -> i32 {
        let mut count = 0;
        for item in &mut self.items {
            if predicate(item) {
                count += item.count();
                *item = ItemStack::empty();
            }
        }
        if count > 0 {
            self.set_changed();
        }
        count
    }
}
