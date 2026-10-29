//! Stonecutter recipe selection shared by the menu and its result slot.

use vecta_registry::REGISTRY;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::recipe::{SingleItemRecipeInput, vanilla_recipe_types};
use vecta_utils::locks::Shared;

use crate::inventory::container::ResultContainer;
use crate::inventory::lock::{ContainerId, ContainerLockGuard};
use crate::inventory::recipe_manager::StonecuttingRecipeRef;

/// The stonecutting recipes offered for the current input plus the selected
/// index.
///
/// The menu owns selection changes and the result slot refills itself from this
/// state after a take, so both sides read one list instead of keeping copies.
pub struct StonecutterSelection {
    recipes: Vec<StonecuttingRecipeRef>,
    selected: i32,
}

impl StonecutterSelection {
    /// Creates an empty selection. The index starts at `0` like vanilla's
    /// standalone data slot; an empty recipe list still produces no result.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            recipes: Vec::new(),
            selected: 0,
        }
    }

    /// Rebuilds the offered recipes for `input` and clears the selection,
    /// matching `StonecutterMenu.setupRecipeList`.
    pub fn rebuild(&mut self, input: &ItemStack) {
        self.selected = -1;
        self.recipes = if input.is_empty() {
            Vec::new()
        } else {
            REGISTRY
                .recipes
                .matching(
                    &vanilla_recipe_types::STONECUTTING,
                    &SingleItemRecipeInput::new(input.clone()),
                )
                .collect()
        };
    }

    /// The selected index, or `-1` when nothing is selected.
    #[must_use]
    pub const fn selected(&self) -> i32 {
        self.selected
    }

    /// Selects the recipe at `button_id`, returning the value to publish in the
    /// menu data slot. Out-of-range ids leave the selection untouched.
    pub fn select(&mut self, button_id: i32) -> Option<i16> {
        if !self.is_valid_index(button_id) {
            return None;
        }
        let index = i16::try_from(button_id).ok()?;
        self.selected = button_id;
        Some(index)
    }

    /// The stack produced by the selected recipe, or an empty stack.
    #[must_use]
    pub fn result(&self) -> ItemStack {
        usize::try_from(self.selected)
            .ok()
            .and_then(|index| self.recipes.get(index))
            .map_or_else(ItemStack::empty, |recipe| recipe.data().result.create())
    }

    /// Whether `button_id` indexes an offered recipe.
    #[must_use]
    pub fn is_valid_index(&self, button_id: i32) -> bool {
        usize::try_from(button_id).is_ok_and(|index| index < self.recipes.len())
    }

    /// Whether any vanilla stonecutting recipe accepts `stack` as input.
    ///
    /// Vanilla asks the full stonecutter recipe set (`acceptsInput`), not the
    /// list filtered by the current input.
    #[must_use]
    pub fn accepts_input(stack: &ItemStack) -> bool {
        !stack.is_empty()
            && REGISTRY
                .recipes
                .matching(
                    &vanilla_recipe_types::STONECUTTING,
                    &SingleItemRecipeInput::new(stack.clone()),
                )
                .next()
                .is_some()
    }
}

impl Default for StonecutterSelection {
    fn default() -> Self {
        Self::new()
    }
}

/// Writes a stack into a stonecutter's virtual result container.
pub fn write_result(
    guard: &mut ContainerLockGuard,
    result_container: &Shared<ResultContainer>,
    stack: ItemStack,
) {
    let Some(result) = guard.get_mut(ContainerId::from_arc(result_container)) else {
        log::warn!("result container not locked while updating the stonecutter result");
        return;
    };
    result.set_item(0, stack);
}
