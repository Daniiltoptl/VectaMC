//! Vanilla `SlotDisplay`, `SelectableRecipe` and `RecipePropertySet` wire types.
//!
//! These are the payloads of `ClientboundUpdateRecipesPacket`: the client filters
//! its stonecutter and recipe-book UI from them, so the server has to encode the
//! exact 26.3 shape.

use std::io::{Cursor, Error, Result, Write};

use vecta_utils::Identifier;
use vecta_utils::codec::VarInt;
use vecta_utils::serial::{ReadFrom, WriteTo};

use crate::items::{Item, ItemRef};
use crate::recipe::Ingredient;
use crate::registry::RegistryEntry as _;
use crate::trim_pattern::TrimPattern;
use crate::{ItemStackTemplate, REGISTRY, RegistryExt, RegistryHolder, RegistryHolderSet};

/// Registry id of a `SlotDisplay` variant.
///
/// The order is `SlotDisplays.bootstrap` in the 26.3 sources
/// (`world/item/crafting/display/SlotDisplays.java`); the wire format is
/// `ByteBufCodecs.registry(Registries.SLOT_DISPLAY).dispatch(...)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum SlotDisplayKind {
    /// `minecraft:empty`.
    Empty = 0,
    /// `minecraft:any_fuel`.
    AnyFuel = 1,
    /// `minecraft:with_any_potion`.
    WithAnyPotion = 2,
    /// `minecraft:only_with_component`.
    OnlyWithComponent = 3,
    /// `minecraft:item`.
    Item = 4,
    /// `minecraft:item_stack`.
    ItemStack = 5,
    /// `minecraft:tag`.
    Tag = 6,
    /// `minecraft:dyed`.
    Dyed = 7,
    /// `minecraft:smithing_trim`.
    SmithingTrim = 8,
    /// `minecraft:with_remainder`.
    WithRemainder = 9,
    /// `minecraft:composite`.
    Composite = 10,
}

impl SlotDisplayKind {
    const fn from_id(id: i32) -> Option<Self> {
        match id {
            0 => Some(Self::Empty),
            1 => Some(Self::AnyFuel),
            2 => Some(Self::WithAnyPotion),
            3 => Some(Self::OnlyWithComponent),
            4 => Some(Self::Item),
            5 => Some(Self::ItemStack),
            6 => Some(Self::Tag),
            7 => Some(Self::Dyed),
            8 => Some(Self::SmithingTrim),
            9 => Some(Self::WithRemainder),
            10 => Some(Self::Composite),
            _ => None,
        }
    }
}

/// Vanilla `SlotDisplay`: one recipe preview slot.
#[derive(Debug, Clone, PartialEq)]
pub enum SlotDisplay {
    /// Shows nothing.
    Empty,
    /// Any item with a cooking-fuel component.
    AnyFuel,
    /// Every potion of the wrapped display.
    WithAnyPotion(Box<SlotDisplay>),
    /// Only the stacks of `source` that carry `component`.
    OnlyWithComponent {
        /// The wrapped display.
        source: Box<SlotDisplay>,
        /// Component key the stack must carry.
        component: Identifier,
    },
    /// A single item type.
    Item(ItemRef),
    /// A full item stack template.
    ItemStack(ItemStackTemplate),
    /// Every item of a named or direct holder set.
    Tag(RegistryHolderSet<Item>),
    /// `target` dyed with the dye stacks of `dye`.
    Dyed {
        /// The dye display.
        dye: Box<SlotDisplay>,
        /// The target display.
        target: Box<SlotDisplay>,
    },
    /// `base` with the trim of `material` and `pattern`.
    SmithingTrim {
        /// The base display.
        base: Box<SlotDisplay>,
        /// The material display.
        material: Box<SlotDisplay>,
        /// The trim pattern.
        pattern: RegistryHolder<TrimPattern>,
    },
    /// `input` paired with its crafting remainder.
    WithRemainder {
        /// The input display.
        input: Box<SlotDisplay>,
        /// The remainder display.
        remainder: Box<SlotDisplay>,
    },
    /// Concatenation of several displays.
    Composite(Vec<SlotDisplay>),
}

impl SlotDisplay {
    /// Returns the registry kind written on the wire.
    #[must_use]
    pub const fn kind(&self) -> SlotDisplayKind {
        match self {
            Self::Empty => SlotDisplayKind::Empty,
            Self::AnyFuel => SlotDisplayKind::AnyFuel,
            Self::WithAnyPotion(_) => SlotDisplayKind::WithAnyPotion,
            Self::OnlyWithComponent { .. } => SlotDisplayKind::OnlyWithComponent,
            Self::Item(_) => SlotDisplayKind::Item,
            Self::ItemStack(_) => SlotDisplayKind::ItemStack,
            Self::Tag(_) => SlotDisplayKind::Tag,
            Self::Dyed { .. } => SlotDisplayKind::Dyed,
            Self::SmithingTrim { .. } => SlotDisplayKind::SmithingTrim,
            Self::WithRemainder { .. } => SlotDisplayKind::WithRemainder,
            Self::Composite(_) => SlotDisplayKind::Composite,
        }
    }
}

impl WriteTo for SlotDisplay {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        VarInt(self.kind() as i32).write(writer)?;
        match self {
            Self::Empty | Self::AnyFuel => Ok(()),
            Self::WithAnyPotion(display) => display.write(writer),
            Self::OnlyWithComponent { source, component } => {
                source.write(writer)?;
                let id = REGISTRY
                    .data_components
                    .id_from_key(component)
                    .ok_or_else(|| Error::other(format!("Unknown component key: {component}")))?;
                VarInt(i32::try_from(id).map_err(|_| Error::other("Component id overflow"))?)
                    .write(writer)
            }
            Self::Item(item) => write_item_id(item, writer),
            Self::ItemStack(template) => template.write(writer),
            Self::Tag(tag) => tag.write(writer),
            Self::Dyed { dye, target } => {
                dye.write(writer)?;
                target.write(writer)
            }
            Self::SmithingTrim {
                base,
                material,
                pattern,
            } => {
                base.write(writer)?;
                material.write(writer)?;
                pattern.write(writer)
            }
            Self::WithRemainder { input, remainder } => {
                input.write(writer)?;
                remainder.write(writer)
            }
            Self::Composite(displays) => displays.write(writer),
        }
    }
}

impl ReadFrom for SlotDisplay {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let kind = SlotDisplayKind::from_id(VarInt::read(data)?.0)
            .ok_or_else(|| Error::other("Unknown slot display kind"))?;
        Ok(match kind {
            SlotDisplayKind::Empty => Self::Empty,
            SlotDisplayKind::AnyFuel => Self::AnyFuel,
            SlotDisplayKind::WithAnyPotion => Self::WithAnyPotion(Box::new(Self::read(data)?)),
            SlotDisplayKind::OnlyWithComponent => {
                let source = Box::new(Self::read(data)?);
                let id = usize::try_from(VarInt::read(data)?.0)
                    .map_err(|_| Error::other("Negative component id"))?;
                let component = REGISTRY
                    .data_components
                    .by_id(id)
                    .map(|entry| entry.key.clone())
                    .ok_or_else(|| Error::other(format!("Unknown component id: {id}")))?;
                Self::OnlyWithComponent { source, component }
            }
            SlotDisplayKind::Item => {
                let id = usize::try_from(VarInt::read(data)?.0)
                    .map_err(|_| Error::other("Negative item id"))?;
                let item = REGISTRY
                    .items
                    .by_id(id)
                    .ok_or_else(|| Error::other(format!("Unknown item id: {id}")))?;
                Self::Item(item)
            }
            SlotDisplayKind::ItemStack => Self::ItemStack(ItemStackTemplate::read(data)?),
            SlotDisplayKind::Tag => Self::Tag(RegistryHolderSet::read(data)?),
            SlotDisplayKind::Dyed => {
                let dye = Box::new(Self::read(data)?);
                let target = Box::new(Self::read(data)?);
                Self::Dyed { dye, target }
            }
            SlotDisplayKind::SmithingTrim => {
                let base = Box::new(Self::read(data)?);
                let material = Box::new(Self::read(data)?);
                let pattern = RegistryHolder::read(data)?;
                Self::SmithingTrim {
                    base,
                    material,
                    pattern,
                }
            }
            SlotDisplayKind::WithRemainder => {
                let input = Box::new(Self::read(data)?);
                let remainder = Box::new(Self::read(data)?);
                Self::WithRemainder { input, remainder }
            }
            SlotDisplayKind::Composite => {
                let count = VarInt::read(data)?.0;
                if count < 0 {
                    return Err(Error::other("Negative composite size"));
                }
                let mut displays = Vec::with_capacity(usize::try_from(count).unwrap_or(0).min(64));
                for _ in 0..count {
                    displays.push(Self::read(data)?);
                }
                Self::Composite(displays)
            }
        })
    }
}

/// One entry of a `SelectableRecipe.SingleInputSet`.
///
/// The recipe itself is never sent: Vanilla's `noRecipeCodec` writes only the
/// option display and an empty recipe.
#[derive(Debug, Clone)]
pub struct SingleInputEntry {
    /// Ingredient that selects this entry for the held input.
    pub input: RegistryHolderSet<Item>,
    /// The preview shown for the result.
    pub option_display: SlotDisplay,
}

impl SingleInputEntry {
    /// Builds a wire entry from a registered recipe's ingredient and preview.
    #[must_use]
    pub fn from_ingredient(input: &Ingredient, option_display: SlotDisplay) -> Self {
        let input = match input {
            Ingredient::Empty => RegistryHolderSet::Direct(Vec::new()),
            Ingredient::Item(item) => RegistryHolderSet::Direct(vec![*item]),
            Ingredient::Tag(tag) => RegistryHolderSet::Tag(tag.clone()),
            Ingredient::Choice(items) => RegistryHolderSet::Direct(items.to_vec()),
        };
        Self {
            input,
            option_display,
        }
    }
}

impl WriteTo for SingleInputEntry {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.input.write(writer)?;
        self.option_display.write(writer)
    }
}

impl ReadFrom for SingleInputEntry {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        Ok(Self {
            input: RegistryHolderSet::read(data)?,
            option_display: SlotDisplay::read(data)?,
        })
    }
}

/// Writes an item registry id the way `Item.STREAM_CODEC` does.
fn write_item_id(item: ItemRef, writer: &mut impl Write) -> Result<()> {
    let id = i32::try_from(item.id()).map_err(|_| Error::other("Item id overflow"))?;
    VarInt(id).write(writer)
}

/// Vanilla `RecipePropertySet`: the item set a property-set key selects.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecipePropertySet {
    items: Vec<ItemRef>,
}

impl RecipePropertySet {
    /// Vanilla `RecipePropertySet.EMPTY`.
    #[must_use]
    pub const fn empty() -> Self {
        Self { items: Vec::new() }
    }

    /// Builds the set from the ingredients Vanilla collects for one key.
    ///
    /// Vanilla stores the flattened item holders of every matching ingredient
    /// in a set; duplicates are therefore dropped while first-seen order is kept.
    #[must_use]
    pub fn from_ingredients<'a>(ingredients: impl IntoIterator<Item = &'a Ingredient>) -> Self {
        let mut items = Vec::new();
        for ingredient in ingredients {
            for item in ingredient.get_items() {
                if !items.contains(&item) {
                    items.push(item);
                }
            }
        }
        Self { items }
    }

    /// Returns whether the set contains the item.
    #[must_use]
    pub fn contains(&self, item: ItemRef) -> bool {
        self.items.contains(&item)
    }

    /// Returns the items of the set.
    #[must_use]
    pub fn items(&self) -> &[ItemRef] {
        &self.items
    }
}

impl WriteTo for RecipePropertySet {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        // `Item.STREAM_CODEC.apply(ByteBufCodecs.list())`: VarInt count, then ids.
        let count =
            i32::try_from(self.items.len()).map_err(|_| Error::other("Property set too large"))?;
        VarInt(count).write(writer)?;
        for item in &self.items {
            write_item_id(item, writer)?;
        }
        Ok(())
    }
}

impl ReadFrom for RecipePropertySet {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let count = VarInt::read(data)?.0;
        if count < 0 {
            return Err(Error::other("Negative property set size"));
        }
        let mut items = Vec::with_capacity(usize::try_from(count).unwrap_or(0).min(1024));
        for _ in 0..count {
            let id = usize::try_from(VarInt::read(data)?.0)
                .map_err(|_| Error::other("Negative item id"))?;
            items.push(
                REGISTRY
                    .items
                    .by_id(id)
                    .ok_or_else(|| Error::other(format!("Unknown item id: {id}")))?,
            );
        }
        Ok(Self { items })
    }
}

/// Vanilla `RecipePropertySet` registry keys, in `RecipeManager` map order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipePropertySetKey {
    /// `minecraft:smithing_addition`.
    SmithingAddition,
    /// `minecraft:smithing_base`.
    SmithingBase,
    /// `minecraft:smithing_template`.
    SmithingTemplate,
    /// `minecraft:furnace_input`.
    FurnaceInput,
    /// `minecraft:blast_furnace_input`.
    BlastFurnaceInput,
    /// `minecraft:smoker_input`.
    SmokerInput,
    /// `minecraft:campfire_input`.
    CampfireInput,
    /// `minecraft:brewing_input`.
    BrewingInput,
    /// `minecraft:brewing_reagent`.
    BrewingReagent,
}

impl RecipePropertySetKey {
    /// Every key Vanilla synchronizes, in a fixed order.
    pub const ALL: [Self; 9] = [
        Self::SmithingAddition,
        Self::SmithingBase,
        Self::SmithingTemplate,
        Self::FurnaceInput,
        Self::BlastFurnaceInput,
        Self::SmokerInput,
        Self::CampfireInput,
        Self::BrewingInput,
        Self::BrewingReagent,
    ];

    /// Returns the registry identifier written on the wire.
    #[must_use]
    pub fn identifier(self) -> Identifier {
        let path = match self {
            Self::SmithingAddition => "smithing_addition",
            Self::SmithingBase => "smithing_base",
            Self::SmithingTemplate => "smithing_template",
            Self::FurnaceInput => "furnace_input",
            Self::BlastFurnaceInput => "blast_furnace_input",
            Self::SmokerInput => "smoker_input",
            Self::CampfireInput => "campfire_input",
            Self::BrewingInput => "brewing_input",
            Self::BrewingReagent => "brewing_reagent",
        };
        Identifier::new("minecraft", path)
    }
}
