//! Vanilla `RecipeDisplay`, `RecipeDisplayId` and `RecipeDisplayEntry` wire
//! types (`net.minecraft.world.item.crafting.display`).
//!
//! The client's recipe book renders these payloads from the recipe-book
//! packets, so the server encodes the exact 26.3 shape. Dispatch happens over
//! the built-in `minecraft:recipe_display` registry; the five vanilla kinds
//! are bootstrap-registered by `RecipeDisplays.bootstrap` in this order:
//! `crafting_shapeless`, `crafting_shaped`, `furnace`, `stonecutter`,
//! `smithing`.

use std::io::{Cursor, Error, Result, Write};

use vecta_utils::codec::VarInt;
use vecta_utils::serial::{PrefixedRead, ReadFrom, WriteTo};

use crate::items::Item;
use crate::recipe::{SlotDisplay, properties::RecipeBookCategoryRef};
use crate::{REGISTRY, RegistryExt as _, RegistryHolderSet};

mod types;
pub use types::{RecipeDisplayType, RecipeDisplayTypeRegistry};

/// Built-in payload kinds, dispatched by the recipe display registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeDisplayKind {
    /// `minecraft:crafting_shapeless`.
    CraftingShapeless,
    /// `minecraft:crafting_shaped`.
    CraftingShaped,
    /// `minecraft:furnace`.
    Furnace,
    /// `minecraft:stonecutter`.
    Stonecutter,
    /// `minecraft:smithing`.
    Smithing,
}

impl RecipeDisplayKind {
    pub(crate) const fn display_type(self) -> RecipeDisplayTypeRef {
        use crate::vanilla_recipe_displays as types;
        match self {
            Self::CraftingShapeless => &types::CRAFTING_SHAPELESS,
            Self::CraftingShaped => &types::CRAFTING_SHAPED,
            Self::Furnace => &types::FURNACE,
            Self::Stonecutter => &types::STONECUTTER,
            Self::Smithing => &types::SMITHING,
        }
    }
}

pub type RecipeDisplayTypeRef = &'static RecipeDisplayType;

/// Vanilla `RecipeDisplay`: one recipe preview for the recipe book.
#[derive(Debug, Clone, PartialEq)]
pub enum RecipeDisplay {
    /// `ShapelessCraftingRecipeDisplay`.
    Shapeless {
        /// The ingredient slots in recipe order.
        ingredients: Vec<SlotDisplay>,
        /// The result preview.
        result: SlotDisplay,
        /// The station that runs the recipe.
        crafting_station: SlotDisplay,
    },
    /// `ShapedCraftingRecipeDisplay`. `ingredients.len()` must equal
    /// `width * height`, mirroring the vanilla compact constructor.
    Shaped {
        /// The grid width.
        width: i32,
        /// The grid height.
        height: i32,
        /// The ingredient slots in row-major order.
        ingredients: Vec<SlotDisplay>,
        /// The result preview.
        result: SlotDisplay,
        /// The station that runs the recipe.
        crafting_station: SlotDisplay,
    },
    /// `FurnaceRecipeDisplay`.
    Furnace {
        /// The smeltable input.
        ingredient: SlotDisplay,
        /// The fuel preview.
        fuel: SlotDisplay,
        /// The result preview.
        result: SlotDisplay,
        /// The station that runs the recipe.
        crafting_station: SlotDisplay,
        /// The burn/cook duration in ticks.
        duration: i32,
        /// The experience granted.
        experience: f32,
    },
    /// `StonecutterRecipeDisplay`.
    Stonecutter {
        /// The cut input.
        input: SlotDisplay,
        /// The result preview.
        result: SlotDisplay,
        /// The station that runs the recipe.
        crafting_station: SlotDisplay,
    },
    /// `SmithingRecipeDisplay`.
    Smithing {
        /// The template slot.
        template: SlotDisplay,
        /// The base slot.
        base: SlotDisplay,
        /// The addition slot.
        addition: SlotDisplay,
        /// The result preview.
        result: SlotDisplay,
        /// The station that runs the recipe.
        crafting_station: SlotDisplay,
    },
}

impl RecipeDisplay {
    /// Returns the built-in payload kind of this display.
    #[must_use]
    pub const fn kind(&self) -> RecipeDisplayKind {
        match self {
            Self::Shapeless { .. } => RecipeDisplayKind::CraftingShapeless,
            Self::Shaped { .. } => RecipeDisplayKind::CraftingShaped,
            Self::Furnace { .. } => RecipeDisplayKind::Furnace,
            Self::Stonecutter { .. } => RecipeDisplayKind::Stonecutter,
            Self::Smithing { .. } => RecipeDisplayKind::Smithing,
        }
    }

    /// Returns the registered discriminator for this payload.
    #[must_use]
    pub const fn display_type(&self) -> RecipeDisplayTypeRef {
        self.kind().display_type()
    }

    fn write_vanilla_payload(&self, writer: &mut impl Write) -> Result<()> {
        match self {
            Self::Shapeless {
                ingredients,
                result,
                crafting_station,
            } => {
                ingredients.write(writer)?;
                result.write(writer)?;
                crafting_station.write(writer)
            }
            Self::Shaped {
                width,
                height,
                ingredients,
                result,
                crafting_station,
            } => {
                VarInt(*width).write(writer)?;
                VarInt(*height).write(writer)?;
                ingredients.write(writer)?;
                result.write(writer)?;
                crafting_station.write(writer)
            }
            Self::Furnace {
                ingredient,
                fuel,
                result,
                crafting_station,
                duration,
                experience,
            } => {
                ingredient.write(writer)?;
                fuel.write(writer)?;
                result.write(writer)?;
                crafting_station.write(writer)?;
                VarInt(*duration).write(writer)?;
                experience.write(writer)
            }
            Self::Stonecutter {
                input,
                result,
                crafting_station,
            } => {
                input.write(writer)?;
                result.write(writer)?;
                crafting_station.write(writer)
            }
            Self::Smithing {
                template,
                base,
                addition,
                result,
                crafting_station,
            } => {
                template.write(writer)?;
                base.write(writer)?;
                addition.write(writer)?;
                result.write(writer)?;
                crafting_station.write(writer)
            }
        }
    }

    /// Vanilla `ByteBufCodecs.list()`: a `VarInt` count with an
    /// `Integer.MAX_VALUE` bound followed by the elements.
    fn read_list<T: ReadFrom>(data: &mut Cursor<&[u8]>) -> Result<Vec<T>> {
        Vec::<T>::read_prefixed_bound::<VarInt>(data, i32::MAX as usize)
    }

    fn read_vanilla_payload(kind: RecipeDisplayKind, data: &mut Cursor<&[u8]>) -> Result<Self> {
        Ok(match kind {
            RecipeDisplayKind::CraftingShapeless => Self::Shapeless {
                ingredients: Self::read_list(data)?,
                result: SlotDisplay::read(data)?,
                crafting_station: SlotDisplay::read(data)?,
            },
            RecipeDisplayKind::CraftingShaped => {
                let width = VarInt::read(data)?.0;
                let height = VarInt::read(data)?.0;
                let ingredients = Self::read_list::<SlotDisplay>(data)?;
                // Vanilla: `ingredients.size() != width * height` in i32.
                let expected = width.wrapping_mul(height);
                if matches!(i32::try_from(ingredients.len()), Ok(len) if len != expected) {
                    return Err(Error::other("Invalid shaped recipe display contents"));
                }
                Self::Shaped {
                    width,
                    height,
                    ingredients,
                    result: SlotDisplay::read(data)?,
                    crafting_station: SlotDisplay::read(data)?,
                }
            }
            RecipeDisplayKind::Furnace => Self::Furnace {
                ingredient: SlotDisplay::read(data)?,
                fuel: SlotDisplay::read(data)?,
                result: SlotDisplay::read(data)?,
                crafting_station: SlotDisplay::read(data)?,
                duration: VarInt::read(data)?.0,
                experience: f32::read(data)?,
            },
            RecipeDisplayKind::Stonecutter => Self::Stonecutter {
                input: SlotDisplay::read(data)?,
                result: SlotDisplay::read(data)?,
                crafting_station: SlotDisplay::read(data)?,
            },
            RecipeDisplayKind::Smithing => Self::Smithing {
                template: SlotDisplay::read(data)?,
                base: SlotDisplay::read(data)?,
                addition: SlotDisplay::read(data)?,
                result: SlotDisplay::read(data)?,
                crafting_station: SlotDisplay::read(data)?,
            },
        })
    }
}

impl WriteTo for RecipeDisplay {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        let (id, display_type) = REGISTRY
            .recipe_displays
            .registered_entry_with_id(self.display_type())
            .ok_or_else(|| std::io::Error::other("Recipe display type is not registered"))?;
        let id =
            i32::try_from(id).map_err(|_| std::io::Error::other("Recipe display id overflow"))?;
        VarInt(id).write(writer)?;
        display_type.write_payload(self, writer)
    }
}

impl ReadFrom for RecipeDisplay {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let id = usize::try_from(VarInt::read(data)?.0)
            .map_err(|_| std::io::Error::other("Negative recipe display id"))?;
        let display_type = REGISTRY
            .recipe_displays
            .by_id(id)
            .ok_or_else(|| std::io::Error::other(format!("Unknown recipe display id: {id}")))?;
        display_type.read_payload(data)
    }
}

/// Vanilla `RecipeDisplayId`: the server-assigned numeric recipe identity
/// used by the recipe-book and place-recipe packets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecipeDisplayId {
    pub index: i32,
}

impl WriteTo for RecipeDisplayId {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        VarInt(self.index).write(writer)
    }
}

impl ReadFrom for RecipeDisplayId {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        Ok(Self {
            index: VarInt::read(data)?.0,
        })
    }
}

/// Vanilla `RecipeDisplayEntry`: one recipe-book entry sent to the client.
#[derive(Debug, Clone, PartialEq)]
pub struct RecipeDisplayEntry {
    /// The server-assigned recipe identity.
    pub id: RecipeDisplayId,
    /// The rendered display.
    pub display: RecipeDisplay,
    /// The recipe group (`OptionalInt`; merged tabs on the client).
    pub group: Option<i32>,
    /// The recipe-book category.
    pub category: RecipeBookCategoryRef,
    /// Vanilla `craftingRequirements`: the stacked-ingredient requirements
    /// the client checks for the "can craft" highlight.
    pub crafting_requirements: Option<Vec<RegistryHolderSet<Item>>>,
}

impl WriteTo for RecipeDisplayEntry {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.id.write(writer)?;
        self.display.write(writer)?;
        // `ByteBufCodecs.OPTIONAL_VAR_INT`: 0 = absent, value + 1 otherwise.
        match self.group {
            None => VarInt(0).write(writer)?,
            Some(group) => {
                let encoded = group
                    .checked_add(1)
                    .ok_or_else(|| std::io::Error::other("Recipe group overflow"))?;
                if encoded < 1 {
                    return Err(std::io::Error::other("Negative recipe group"));
                }
                VarInt(encoded).write(writer)?;
            }
        }
        let category_id = REGISTRY
            .recipe_book_categories
            .id_from_key(&self.category.key)
            .ok_or_else(|| Error::other("Recipe book category is not registered"))?;
        VarInt(
            i32::try_from(category_id)
                .map_err(|_| Error::other("Recipe book category id overflow"))?,
        )
        .write(writer)?;
        match &self.crafting_requirements {
            None => false.write(writer)?,
            Some(requirements) => {
                true.write(writer)?;
                requirements.write(writer)?;
            }
        }
        Ok(())
    }
}

impl ReadFrom for RecipeDisplayEntry {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let id = RecipeDisplayId::read(data)?;
        let display = RecipeDisplay::read(data)?;
        let encoded = VarInt::read(data)?.0;
        let group = if encoded == 0 {
            None
        } else {
            Some(encoded - 1)
        };
        let category_id = usize::try_from(VarInt::read(data)?.0)
            .map_err(|_| std::io::Error::other("Negative recipe book category id"))?;
        let category = REGISTRY
            .recipe_book_categories
            .by_id(category_id)
            .ok_or_else(|| {
                std::io::Error::other(format!("Unknown recipe book category id: {category_id}"))
            })?;
        let crafting_requirements = if bool::read(data)? {
            Some(RecipeDisplay::read_list::<RegistryHolderSet<Item>>(data)?)
        } else {
            None
        };
        Ok(Self {
            id,
            display,
            group,
            category,
            crafting_requirements,
        })
    }
}

#[cfg(test)]
mod tests;
