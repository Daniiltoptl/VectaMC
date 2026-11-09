//! Passive single-item recipe data and input snapshots.

use vecta_utils::{DowncastType, DowncastTypeKey, Identifier};

use crate::item_stack::ItemStack;
use crate::item_stack_template::ItemStackTemplate;
use crate::recipe::display::SlotDisplay;
use crate::recipe::recipe_display::RecipeDisplay;
use crate::vanilla_items;

use super::{Ingredient, RecipeData, RecipeInput, RecipeMatches, RecipeProperties};

/// Input shared by cooking and stonecutting recipe types.
#[derive(Debug, Clone)]
pub struct SingleItemRecipeInput {
    pub item: ItemStack,
}

impl SingleItemRecipeInput {
    #[must_use]
    pub const fn new(item: ItemStack) -> Self {
        Self { item }
    }
}

// SAFETY: This Vecta-owned key uniquely identifies a single-item matching snapshot.
unsafe impl DowncastType for SingleItemRecipeInput {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:recipe_input/single_item");
}

impl RecipeInput for SingleItemRecipeInput {
    fn is_empty(&self) -> bool {
        self.item.is_empty()
    }
}

/// Data shared by smelting, blasting, smoking, and campfire cooking.
#[derive(Debug)]
pub struct CookingRecipe {
    pub properties: RecipeProperties,
    pub ingredient: Ingredient,
    pub result: ItemStackTemplate,
    pub experience: f32,
    pub cooking_time: i32,
}

// SAFETY: This Vecta-owned key uniquely identifies vanilla cooking recipe data.
unsafe impl DowncastType for CookingRecipe {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:recipe_data/cooking");
}

impl RecipeData for CookingRecipe {
    fn properties(&self) -> Option<&RecipeProperties> {
        Some(&self.properties)
    }
}

impl CookingRecipe {
    /// Builds the recipe-book display, mirroring
    /// `AbstractCookingRecipe.display()` with the serializer's `furnaceIcon`.
    #[must_use]
    pub fn displays(&self, recipe_type: &Identifier) -> Vec<RecipeDisplay> {
        fn furnace_icon(recipe_type: &Identifier) -> SlotDisplay {
            match recipe_type.path.as_ref() {
                "blasting" => SlotDisplay::Item(&vanilla_items::BLAST_FURNACE),
                "smoking" => SlotDisplay::Item(&vanilla_items::SMOKER),
                "campfire_cooking" => SlotDisplay::Item(&vanilla_items::CAMPFIRE),
                _ => SlotDisplay::Item(&vanilla_items::FURNACE),
            }
        }
        vec![RecipeDisplay::Furnace {
            ingredient: self.ingredient.display(),
            fuel: SlotDisplay::AnyFuel,
            result: SlotDisplay::ItemStack(self.result.clone()),
            crafting_station: furnace_icon(recipe_type),
            duration: self.cooking_time,
            experience: self.experience,
        }]
    }
}

/// Stonecutter recipe data.
#[derive(Debug)]
pub struct StonecuttingRecipe {
    pub properties: RecipeProperties,
    pub ingredient: Ingredient,
    pub result: ItemStackTemplate,
}

// SAFETY: This Vecta-owned key uniquely identifies vanilla stonecutting recipe data.
unsafe impl DowncastType for StonecuttingRecipe {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:recipe_data/stonecutting");
}

impl RecipeData for StonecuttingRecipe {
    fn properties(&self) -> Option<&RecipeProperties> {
        Some(&self.properties)
    }
}

impl StonecuttingRecipe {
    /// Builds the recipe-book display, mirroring
    /// `StonecutterRecipe.display()`. This is book-only; the stonecutter
    /// menu keeps its own update-recipes path.
    #[must_use]
    pub fn displays(&self) -> Vec<RecipeDisplay> {
        vec![RecipeDisplay::Stonecutter {
            input: self.ingredient.display(),
            result: SlotDisplay::ItemStack(self.result.clone()),
            crafting_station: SlotDisplay::Item(&vanilla_items::STONECUTTER),
        }]
    }
}

impl RecipeMatches<SingleItemRecipeInput> for CookingRecipe {
    fn matches(&self, input: &SingleItemRecipeInput) -> bool {
        self.ingredient.test(&input.item)
    }
}

impl RecipeMatches<SingleItemRecipeInput> for StonecuttingRecipe {
    fn matches(&self, input: &SingleItemRecipeInput) -> bool {
        self.ingredient.test(&input.item)
    }
}
