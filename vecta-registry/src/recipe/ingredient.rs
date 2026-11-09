//! Ingredient matching for recipes.

use vecta_utils::Identifier;

use super::display::SlotDisplay;
use crate::items::{Item, ItemRef};
use crate::{REGISTRY, RegistryHolderSet, TaggedRegistryExt, item_stack::ItemStack};

/// Represents what items can satisfy a recipe slot.
/// Matches Java's `Ingredient` class.
#[derive(Debug, Clone)]
pub enum Ingredient {
    /// Matches nothing (empty slot required).
    Empty,
    /// Matches a single specific item.
    Item(ItemRef),
    /// Matches any item in a tag.
    Tag(Identifier),
    /// Matches any of the listed items (OR).
    Choice(&'static [ItemRef]),
}

impl Ingredient {
    /// Tests if the given item stack satisfies this ingredient.
    #[must_use]
    pub fn test(&self, stack: &ItemStack) -> bool {
        match self {
            Self::Empty => stack.is_empty(),
            Self::Item(item) => !stack.is_empty() && *item == stack.item,
            Self::Tag(tag) => {
                if stack.is_empty() {
                    return false;
                }
                REGISTRY.items.is_in_tag(stack.item, tag)
            }
            Self::Choice(items) => {
                if stack.is_empty() {
                    return false;
                }
                items.contains(&stack.item)
            }
        }
    }

    /// Returns true if this ingredient matches an empty slot.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Returns all items that could satisfy this ingredient (for recipe book display).
    #[must_use]
    pub fn get_items(&self) -> Vec<ItemRef> {
        match self {
            Self::Empty => Vec::new(),
            Self::Item(item) => vec![*item],
            Self::Tag(tag) => REGISTRY.items.get_tag(tag).unwrap_or_default(),
            Self::Choice(items) => items.to_vec(),
        }
    }

    /// Compares two ingredients for equality (used for symmetry detection).
    #[must_use]
    pub fn eq_ingredient(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty) => true,
            (Self::Item(a), Self::Item(b)) => a == b,
            (Self::Tag(a), Self::Tag(b)) => a == b,
            (Self::Choice(a), Self::Choice(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x == y)
            }
            _ => false,
        }
    }

    /// Builds the recipe-book slot display for this ingredient.
    ///
    /// Vanilla always renders an ingredient as a `TagSlotDisplay` over its
    /// holder set; a Rust `Empty` ingredient corresponds to the vanilla
    /// `Optional.empty` slot, which recipes map to `SlotDisplay.Empty`.
    #[must_use]
    pub fn display(&self) -> SlotDisplay {
        match self {
            Self::Empty => SlotDisplay::Empty,
            Self::Item(item) => SlotDisplay::Tag(RegistryHolderSet::Direct(vec![*item])),
            Self::Tag(tag) => SlotDisplay::Tag(RegistryHolderSet::Tag(tag.clone())),
            Self::Choice(items) => SlotDisplay::Tag(RegistryHolderSet::Direct(items.to_vec())),
        }
    }

    /// Builds the stacked-ingredients holder set used by
    /// `RecipeDisplayEntry.craftingRequirements` (vanilla `Ingredient`'s
    /// `CONTENTS_STREAM_CODEC` payload).
    #[must_use]
    pub fn holder_set(&self) -> RegistryHolderSet<Item> {
        match self {
            Self::Empty => RegistryHolderSet::Direct(Vec::new()),
            Self::Item(item) => RegistryHolderSet::Direct(vec![*item]),
            Self::Tag(tag) => RegistryHolderSet::Tag(tag.clone()),
            Self::Choice(items) => RegistryHolderSet::Direct(items.to_vec()),
        }
    }
}
