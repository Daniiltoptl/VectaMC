//! Server-side recipe book state (vanilla `RecipeBook`,
//! `RecipeBookSettings` and `ServerRecipeBook`).
//!
//! Known/highlight recipe keys and the per-menu-type tab settings live on the
//! player. The book never stores packet entries: displays are resolved from
//! the [`DisplayIndex`](super::registry::DisplayIndex) at send time, so a
//! changed recipe set cannot leave stale ids behind. Persistence is the
//! vanilla `ServerRecipeBook.Packed` codec: `settings`, `recipes` (known) and
//! `toBeDisplayed` (highlight).

use rustc_hash::FxHashSet;
use simdnbt::owned::{NbtCompound, NbtList, NbtTag};
use simdnbt::{FromNbtTag, ToNbtTag};
use std::io::{Cursor, Result, Write};
use vecta_utils::Identifier;
use vecta_utils::serial::{ReadFrom, WriteTo};

use super::registry::UntypedRecipeRef;

/// The vanilla `RecipeBookType` menus that keep their own tab state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeBookType {
    Crafting,
    Furnace,
    BlastFurnace,
    Smoker,
}

/// Open/filtering state of one menu type (vanilla `RecipeBookSettings.TypeSettings`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TypeSettings {
    pub open: bool,
    pub filtering: bool,
}

impl TypeSettings {
    /// Vanilla wire default: both flags false.
    pub const DEFAULT: Self = Self {
        open: false,
        filtering: false,
    };

    /// Vanilla `setOpen`.
    #[must_use]
    pub const fn set_open(self, open: bool) -> Self {
        Self {
            open,
            filtering: self.filtering,
        }
    }

    /// Vanilla `setFiltering`.
    #[must_use]
    pub const fn set_filtering(self, filtering: bool) -> Self {
        Self {
            open: self.open,
            filtering,
        }
    }
}

/// Per-menu-type recipe book tabs (vanilla `RecipeBookSettings`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecipeBookSettings {
    crafting: TypeSettings,
    furnace: TypeSettings,
    blast_furnace: TypeSettings,
    smoker: TypeSettings,
}

impl Default for RecipeBookSettings {
    fn default() -> Self {
        Self {
            crafting: TypeSettings::DEFAULT,
            furnace: TypeSettings::DEFAULT,
            blast_furnace: TypeSettings::DEFAULT,
            smoker: TypeSettings::DEFAULT,
        }
    }
}

impl RecipeBookSettings {
    /// Vanilla `getSettings(type)`.
    #[must_use]
    pub const fn get(&self, book_type: RecipeBookType) -> &TypeSettings {
        match book_type {
            RecipeBookType::Crafting => &self.crafting,
            RecipeBookType::Furnace => &self.furnace,
            RecipeBookType::BlastFurnace => &self.blast_furnace,
            RecipeBookType::Smoker => &self.smoker,
        }
    }

    const fn get_mut(&mut self, book_type: RecipeBookType) -> &mut TypeSettings {
        match book_type {
            RecipeBookType::Crafting => &mut self.crafting,
            RecipeBookType::Furnace => &mut self.furnace,
            RecipeBookType::BlastFurnace => &mut self.blast_furnace,
            RecipeBookType::Smoker => &mut self.smoker,
        }
    }

    /// Vanilla `setOpen`.
    pub const fn set_open(&mut self, book_type: RecipeBookType, open: bool) {
        *self.get_mut(book_type) = self.get(book_type).set_open(open);
    }

    /// Vanilla `setFiltering`.
    pub const fn set_filtering(&mut self, book_type: RecipeBookType, filtering: bool) {
        *self.get_mut(book_type) = self.get(book_type).set_filtering(filtering);
    }

    /// Vanilla `RecipeBook.setBookSetting`.
    pub const fn set_book_setting(
        &mut self,
        book_type: RecipeBookType,
        open: bool,
        filtering: bool,
    ) {
        *self.get_mut(book_type) = TypeSettings { open, filtering };
    }

    /// Vanilla `replaceFrom`.
    pub const fn replace_from(&mut self, other: &Self) {
        *self = *other;
    }
}

impl WriteTo for RecipeBookSettings {
    /// Vanilla `RecipeBookSettings.STREAM_CODEC`: four `TypeSettings` pairs in
    /// crafting, furnace, blast furnace, smoker order.
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        for book_type in [
            RecipeBookType::Crafting,
            RecipeBookType::Furnace,
            RecipeBookType::BlastFurnace,
            RecipeBookType::Smoker,
        ] {
            let settings = self.get(book_type);
            settings.open.write(writer)?;
            settings.filtering.write(writer)?;
        }
        Ok(())
    }
}

impl ReadFrom for RecipeBookSettings {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let mut settings = Self::default();
        for book_type in [
            RecipeBookType::Crafting,
            RecipeBookType::Furnace,
            RecipeBookType::BlastFurnace,
            RecipeBookType::Smoker,
        ] {
            let open = bool::read(data)?;
            let filtering = bool::read(data)?;
            settings.set_book_setting(book_type, open, filtering);
        }
        Ok(settings)
    }
}

/// The persisted book (vanilla `ServerRecipeBook.Packed`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packed {
    pub settings: RecipeBookSettings,
    /// `recipes`: every known recipe key.
    pub known: Vec<Identifier>,
    /// `toBeDisplayed`: the keys still highlighted for the client.
    pub highlight: Vec<Identifier>,
}

impl Packed {
    fn to_compound(&self) -> NbtCompound {
        let mut settings = NbtCompound::new();
        for (open_key, filtering_key, book_type) in [
            (
                "isGuiOpen",
                "isFilteringCraftable",
                RecipeBookType::Crafting,
            ),
            (
                "isFurnaceGuiOpen",
                "isFurnaceFilteringCraftable",
                RecipeBookType::Furnace,
            ),
            (
                "isBlastingFurnaceGuiOpen",
                "isBlastingFurnaceFilteringCraftable",
                RecipeBookType::BlastFurnace,
            ),
            (
                "isSmokerGuiOpen",
                "isSmokerFilteringCraftable",
                RecipeBookType::Smoker,
            ),
        ] {
            let settings_of_type = self.settings.get(book_type);
            settings.insert(open_key, NbtTag::Byte(i8::from(settings_of_type.open)));
            settings.insert(
                filtering_key,
                NbtTag::Byte(i8::from(settings_of_type.filtering)),
            );
        }
        let mut compound = NbtCompound::new();
        compound.insert("settings", NbtTag::Compound(settings));
        compound.insert(
            "recipes",
            NbtTag::List(NbtList::from(
                self.known
                    .iter()
                    .map(Identifier::to_string)
                    .collect::<Vec<_>>(),
            )),
        );
        compound.insert(
            "toBeDisplayed",
            NbtTag::List(NbtList::from(
                self.highlight
                    .iter()
                    .map(Identifier::to_string)
                    .collect::<Vec<_>>(),
            )),
        );
        compound
    }

    fn from_compound(compound: &simdnbt::borrow::NbtCompound) -> Option<Self> {
        let mut settings = RecipeBookSettings::default();
        let stored = compound.get("settings")?.compound()?;
        for (open_key, filtering_key, book_type) in [
            (
                "isGuiOpen",
                "isFilteringCraftable",
                RecipeBookType::Crafting,
            ),
            (
                "isFurnaceGuiOpen",
                "isFurnaceFilteringCraftable",
                RecipeBookType::Furnace,
            ),
            (
                "isBlastingFurnaceGuiOpen",
                "isBlastingFurnaceFilteringCraftable",
                RecipeBookType::BlastFurnace,
            ),
            (
                "isSmokerGuiOpen",
                "isSmokerFilteringCraftable",
                RecipeBookType::Smoker,
            ),
        ] {
            let open = stored
                .get(open_key)
                .and_then(|tag| tag.byte())
                .is_some_and(|value| value != 0);
            let filtering = stored
                .get(filtering_key)
                .and_then(|tag| tag.byte())
                .is_some_and(|value| value != 0);
            settings.set_book_setting(book_type, open, filtering);
        }
        let key_list = |tag: Option<simdnbt::borrow::NbtTag>| -> Option<Vec<Identifier>> {
            tag.map_or_else(
                || Some(Vec::new()),
                |tag| {
                    tag.list()?
                        .to_owned()
                        .as_nbt_tags()
                        .iter()
                        .map(|tag| tag.string()?.to_str().parse::<Identifier>().ok())
                        .collect()
                },
            )
        };
        Some(Self {
            settings,
            known: key_list(compound.get("recipes"))?,
            highlight: key_list(compound.get("toBeDisplayed"))?,
        })
    }
}

impl ToNbtTag for Packed {
    fn to_nbt_tag(self) -> NbtTag {
        NbtTag::Compound(self.to_compound())
    }
}

impl FromNbtTag for Packed {
    fn from_nbt_tag(tag: simdnbt::borrow::NbtTag) -> Option<Self> {
        Self::from_compound(&tag.compound()?)
    }
}

/// Server-side book state (vanilla `ServerRecipeBook`).
///
/// `known` holds every recipe the player has unlocked; `highlight` holds the
/// subset still marked as new. Both are keyed by the recipe identifier, so
/// entries survive logout/login and respawns without being tied to a menu
/// instance.
#[derive(Debug, Default)]
pub struct ServerRecipeBook {
    known: FxHashSet<Identifier>,
    highlight: FxHashSet<Identifier>,
    book_settings: RecipeBookSettings,
}

impl ServerRecipeBook {
    /// Vanilla `add`.
    pub fn add(&mut self, key: &Identifier) {
        self.known.insert(key.clone());
    }

    /// Vanilla `contains`.
    #[must_use]
    pub fn contains(&self, key: &Identifier) -> bool {
        self.known.contains(key)
    }

    /// Vanilla `removeHighlight`.
    pub fn remove_highlight(&mut self, key: &Identifier) {
        self.highlight.remove(key);
    }

    /// Vanilla `remove`.
    pub fn remove(&mut self, key: &Identifier) {
        self.known.remove(key);
        self.highlight.remove(key);
    }

    /// Vanilla `RecipeBook` settings accessors.
    #[must_use]
    pub const fn book_settings(&self) -> &RecipeBookSettings {
        &self.book_settings
    }

    /// Vanilla `RecipeBook.setBookSettings`.
    pub const fn set_book_settings(&mut self, settings: &RecipeBookSettings) {
        self.book_settings.replace_from(settings);
    }

    /// Vanilla `RecipeBook.setBookSetting`.
    pub const fn set_book_setting(
        &mut self,
        book_type: RecipeBookType,
        open: bool,
        filtering: bool,
    ) {
        self.book_settings
            .set_book_setting(book_type, open, filtering);
    }

    /// The highlight membership of a known recipe.
    #[must_use]
    pub fn is_highlighted(&self, key: &Identifier) -> bool {
        self.highlight.contains(key)
    }

    /// Vanilla `addRecipes` state part: unlocks every not-yet-known,
    /// non-special recipe and returns the newly awarded ones. The caller
    /// resolves their displays into the add packet and triggers the unlock
    /// criteria; special recipes are never added.
    pub fn award_recipes(
        &mut self,
        recipes: impl IntoIterator<Item = UntypedRecipeRef>,
    ) -> Vec<UntypedRecipeRef> {
        let mut awarded = Vec::new();
        for recipe in recipes {
            let Some(properties) = recipe.data().properties() else {
                continue;
            };
            if properties.special {
                continue;
            }
            if self.known.insert(recipe.key().clone()) {
                self.highlight.insert(recipe.key().clone());
                awarded.push(recipe);
            }
        }
        awarded
    }

    /// Vanilla `removeRecipes` state part: forgets the known recipes and
    /// returns those that were known, so the caller can resolve their display
    /// ids into the remove packet.
    pub fn forget_recipes(
        &mut self,
        recipes: impl IntoIterator<Item = UntypedRecipeRef>,
    ) -> Vec<UntypedRecipeRef> {
        recipes
            .into_iter()
            .filter(|recipe| self.known.remove(recipe.key()))
            .inspect(|recipe| {
                self.highlight.remove(recipe.key());
            })
            .collect()
    }

    /// Vanilla `pack`.
    #[must_use]
    pub fn pack(&self) -> Packed {
        Packed {
            settings: self.book_settings,
            known: self.known.iter().cloned().collect(),
            highlight: self.highlight.iter().cloned().collect(),
        }
    }

    /// Vanilla `apply` (trusted data, e.g. `copyOverData` between live books).
    pub fn apply(&mut self, packed: &Packed) {
        self.known = packed.known.iter().cloned().collect();
        self.highlight = packed.highlight.iter().cloned().collect();
        self.book_settings.replace_from(&packed.settings);
    }

    /// Vanilla `loadUntrusted`: restores saved data, silently dropping
    /// unknown recipe keys with an error log like vanilla's `loadRecipes`.
    pub fn load_untrusted(&mut self, packed: &Packed, validator: impl Fn(&Identifier) -> bool) {
        self.book_settings.replace_from(&packed.settings);
        self.load_keys(&packed.known, false, &validator);
        self.load_keys(&packed.highlight, true, &validator);
    }

    fn load_keys(
        &mut self,
        keys: &[Identifier],
        highlight: bool,
        validator: &impl Fn(&Identifier) -> bool,
    ) {
        for key in keys {
            if validator(key) {
                if highlight {
                    self.highlight.insert(key.clone());
                } else {
                    self.known.insert(key.clone());
                }
            } else {
                tracing::error!("Tried to load unrecognized recipe: {} removed now.", key);
            }
        }
    }
}

#[cfg(test)]
mod book_tests {
    use simdnbt::borrow::read_tag;
    use simdnbt::{FromNbtTag as _, ToNbtTag as _};
    use std::io::Cursor;
    use vecta_utils::Identifier;
    use vecta_utils::serial::{ReadFrom as _, WriteTo as _};

    use super::super::registry::UntypedRecipeRef;
    use super::{Packed, RecipeBookSettings, RecipeBookType, ServerRecipeBook, TypeSettings};
    use crate::{REGISTRY, init_vanilla_registry, vanilla_recipes};

    fn potato_ref() -> UntypedRecipeRef {
        REGISTRY
            .recipes
            .by_key(vanilla_recipes::BAKED_POTATO.key())
            .expect("generated baked potato recipe must be registered")
    }

    fn pickaxe_ref() -> UntypedRecipeRef {
        REGISTRY
            .recipes
            .by_key(vanilla_recipes::IRON_PICKAXE.key())
            .expect("generated iron pickaxe recipe must be registered")
    }

    fn write_settings_bytes(settings: &RecipeBookSettings) -> Vec<u8> {
        let mut bytes = Vec::new();
        settings.write(&mut bytes).expect("settings write failed");
        bytes
    }

    #[test]
    fn settings_default_to_closed_menus_and_update_in_place() {
        let mut settings = RecipeBookSettings::default();
        for book_type in [
            RecipeBookType::Crafting,
            RecipeBookType::Furnace,
            RecipeBookType::BlastFurnace,
            RecipeBookType::Smoker,
        ] {
            assert_eq!(
                settings.get(book_type),
                &TypeSettings::DEFAULT,
                "every menu type starts closed with filtering off"
            );
        }

        settings.set_open(RecipeBookType::Furnace, true);
        settings.set_filtering(RecipeBookType::BlastFurnace, true);
        assert_eq!(
            settings.get(RecipeBookType::Furnace),
            &TypeSettings {
                open: true,
                filtering: false
            }
        );
        assert_eq!(
            settings.get(RecipeBookType::BlastFurnace),
            &TypeSettings {
                open: false,
                filtering: true
            }
        );
        assert_eq!(
            settings.get(RecipeBookType::Crafting),
            &TypeSettings::DEFAULT
        );

        settings.set_book_setting(RecipeBookType::Smoker, true, true);
        assert_eq!(
            settings.get(RecipeBookType::Smoker),
            &TypeSettings {
                open: true,
                filtering: true
            }
        );
    }

    #[test]
    fn wire_settings_round_trip_is_eight_bools_in_type_order() {
        let mut settings = RecipeBookSettings::default();
        settings.set_open(RecipeBookType::Crafting, true);
        settings.set_filtering(RecipeBookType::Furnace, true);
        settings.set_filtering(RecipeBookType::Smoker, true);

        let bytes = write_settings_bytes(&settings);
        assert_eq!(
            bytes,
            vec![1, 0, 0, 1, 0, 0, 0, 1],
            "crafting(open), furnace(filtering) and smoker(filtering) must sit at the \
             vanilla STREAM_CODEC positions"
        );

        let mut cursor = Cursor::new(bytes.as_slice());
        let restored = RecipeBookSettings::read(&mut cursor).expect("settings read failed");
        assert_eq!(restored, settings);
        assert_eq!(cursor.position(), 8, "no trailing bytes may remain");
    }

    #[test]
    fn packed_nbt_uses_the_vanilla_field_names() {
        init_vanilla_registry();
        let mut settings = RecipeBookSettings::default();
        settings.set_book_setting(RecipeBookType::BlastFurnace, true, false);
        let key = vanilla_recipes::BAKED_POTATO.key();
        let packed = Packed {
            settings,
            known: vec![key.clone()],
            highlight: vec![key.clone()],
        };

        let expected = packed.clone();
        let tag = packed.to_nbt_tag();
        let mut bytes = Vec::new();
        tag.write(&mut bytes);
        let mut cursor = Cursor::new(bytes.as_slice());
        let parsed = read_tag(&mut cursor).expect("owned compound must re-parse as borrowed");
        let borrowed = parsed.as_tag();
        let compound = borrowed
            .compound()
            .expect("round trip must stay a compound");

        let settings = compound
            .get("settings")
            .and_then(|tag| tag.compound())
            .expect("vanilla saves tabs under the `settings` compound");
        assert_eq!(
            settings
                .get("isBlastingFurnaceGuiOpen")
                .and_then(|tag| tag.byte()),
            Some(1),
            "blast furnace open flag must use the vanilla key"
        );
        assert_eq!(
            settings
                .get("isSmokerFilteringCraftable")
                .and_then(|tag| tag.byte()),
            Some(0),
            "unset flags are written as explicit false bytes"
        );
        assert!(compound.get("recipes").and_then(|tag| tag.list()).is_some());
        assert!(
            compound
                .get("toBeDisplayed")
                .and_then(|tag| tag.list())
                .is_some()
        );

        let restored =
            Packed::from_nbt_tag(borrowed).expect("packed book must round trip through NBT");
        assert_eq!(restored, expected);
    }

    #[test]
    fn award_and_forget_track_known_and_highlight() {
        init_vanilla_registry();
        let potato = potato_ref();
        let pickaxe = pickaxe_ref();
        let mut book = ServerRecipeBook::default();

        let awarded = book.award_recipes([potato, pickaxe]);
        assert_eq!(awarded.len(), 2);
        assert!(awarded.iter().any(|recipe| recipe.key() == potato.key()));
        assert!(awarded.iter().any(|recipe| recipe.key() == pickaxe.key()));
        assert!(book.contains(potato.key()));
        assert!(book.contains(pickaxe.key()));
        assert!(book.is_highlighted(potato.key()));
        assert!(book.is_highlighted(pickaxe.key()));

        let again = book.award_recipes([potato, pickaxe]);
        assert!(
            again.is_empty(),
            "re-awarding known recipes must not re-highlight them"
        );
        assert!(book.is_highlighted(potato.key()));

        let forgotten = book.forget_recipes([potato]);
        assert_eq!(forgotten.len(), 1);
        assert_eq!(forgotten[0].key(), potato.key());
        assert!(!book.contains(potato.key()));
        assert!(!book.is_highlighted(potato.key()));
        assert!(book.contains(pickaxe.key()));

        let nothing = book.forget_recipes([potato]);
        assert!(
            nothing.is_empty(),
            "forgetting an unknown recipe is a no-op"
        );
    }

    #[test]
    fn load_untrusted_drops_unknown_keys_and_keeps_settings() {
        init_vanilla_registry();
        let potato = potato_ref();
        let pickaxe = pickaxe_ref();
        let mut packed = Packed {
            settings: RecipeBookSettings::default(),
            known: vec![
                potato.key().clone(),
                Identifier::new_static("minecraft", "definitely_not_a_recipe"),
            ],
            highlight: vec![pickaxe.key().clone()],
        };
        packed.settings.set_open(RecipeBookType::Crafting, true);

        let mut book = ServerRecipeBook::default();
        book.load_untrusted(&packed, |key| REGISTRY.recipes.by_key(key).is_some());

        assert!(book.contains(potato.key()));
        assert!(!book.contains(&Identifier::new_static(
            "minecraft",
            "definitely_not_a_recipe"
        )));
        assert!(book.is_highlighted(pickaxe.key()));
        assert!(book.book_settings().get(RecipeBookType::Crafting).open);
    }

    #[test]
    fn pack_apply_and_save_load_round_trip_the_whole_book() {
        init_vanilla_registry();
        let potato = potato_ref();
        let pickaxe = pickaxe_ref();
        let mut book = ServerRecipeBook::default();
        book.award_recipes([potato, pickaxe]);
        book.set_book_setting(RecipeBookType::Furnace, true, false);
        book.remove_highlight(potato.key());

        let packed = book.pack();
        assert_eq!(packed.known.len(), 2);
        assert_eq!(packed.highlight.len(), 1);

        let mut restored = ServerRecipeBook::default();
        restored.apply(&packed);
        assert!(restored.contains(potato.key()));
        assert!(restored.contains(pickaxe.key()));
        assert!(!restored.is_highlighted(potato.key()));
        assert!(restored.is_highlighted(pickaxe.key()));
        assert!(restored.book_settings().get(RecipeBookType::Furnace).open);
        assert!(
            !restored
                .book_settings()
                .get(RecipeBookType::Furnace)
                .filtering
        );

        // Full save/load path: pack -> NBT -> Packed -> load_untrusted.
        let tag = packed.to_nbt_tag();
        let mut bytes = Vec::new();
        tag.write(&mut bytes);
        let mut cursor = Cursor::new(bytes.as_slice());
        let parsed = read_tag(&mut cursor).expect("saved book must re-parse");
        let borrowed = parsed.as_tag();
        let loaded =
            Packed::from_nbt_tag(borrowed).expect("saved book must decode as vanilla Packed");
        let mut from_disk = ServerRecipeBook::default();
        from_disk.load_untrusted(&loaded, |key| REGISTRY.recipes.by_key(key).is_some());
        assert!(from_disk.contains(potato.key()));
        assert!(from_disk.contains(pickaxe.key()));
        assert!(!from_disk.is_highlighted(potato.key()));
        assert!(from_disk.is_highlighted(pickaxe.key()));
        assert!(from_disk.book_settings().get(RecipeBookType::Furnace).open);
    }
}
