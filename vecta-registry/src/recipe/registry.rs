//! Heterogeneous recipe storage with typed per-type views.

use rustc_hash::FxHashMap;
use vecta_utils::{Downcast as _, DowncastType, Identifier};

use super::recipe_display::{RecipeDisplay, RecipeDisplayEntry, RecipeDisplayId};
use super::{
    CookingRecipe, CraftingRecipe, ErasedRecipe, Ingredient, Recipe, RecipeData, RecipeInput,
    RecipeMatches, RecipeType, RecipeTypeEntryRef, RecipeTypeRegistry, SmithingRecipe,
    StonecuttingRecipe,
};

/// One unpacked recipe-book entry with its parent recipe (vanilla
/// `RecipeManager.ServerDisplayInfo`).
#[derive(Debug, Clone)]
pub struct ServerDisplayInfo {
    /// The entry sent to the client, carrying the server-assigned id.
    pub display: RecipeDisplayEntry,
    /// The registered recipe this display belongs to.
    pub parent: UntypedRecipeRef,
}

/// Server-side index over all recipe-book displays (vanilla `RecipeManager`'s
/// `allDisplays` and `recipeToDisplay` pair). Built in registration order so
/// that `RecipeDisplayId`s are stable across reserialization.
pub struct DisplayIndex {
    displays: Vec<ServerDisplayInfo>,
    recipe_to_display: FxHashMap<Identifier, Vec<usize>>,
}

impl DisplayIndex {
    /// Unpacks the recipe-book displays (vanilla `RecipeManager.unpackRecipeInfo`):
    /// recipes iterate in registry order, group ids are assigned per unpack by
    /// first occurrence, special recipes carry no `craftingRequirements`, and
    /// every display of a recipe gets a sequential `RecipeDisplayId`.
    ///
    /// Feature flags are not modeled; vanilla recipes declare no
    /// feature-flagged displays.
    #[must_use]
    pub fn build(registry: &RecipeRegistry) -> Self {
        let mut displays: Vec<ServerDisplayInfo> = Vec::new();
        let mut recipe_to_display: FxHashMap<Identifier, Vec<usize>> = FxHashMap::default();
        let mut groups: FxHashMap<String, i32> = FxHashMap::default();
        for recipe in registry.iter() {
            let Some(properties) = recipe.data().properties() else {
                continue;
            };
            let Some(book) = properties.recipe_book.as_ref() else {
                continue;
            };
            let payload_displays: Vec<RecipeDisplay> =
                if let Some(data) = recipe.downcast_data::<CraftingRecipe>() {
                    data.displays()
                } else if let Some(data) = recipe.downcast_data::<CookingRecipe>() {
                    data.displays(&recipe.recipe_type().key)
                } else if let Some(data) = recipe.downcast_data::<StonecuttingRecipe>() {
                    data.displays()
                } else if let Some(data) = recipe.downcast_data::<SmithingRecipe>() {
                    data.displays()
                } else {
                    Vec::new()
                };
            if payload_displays.is_empty() {
                continue;
            }
            let group = book.group.as_ref().map(|group| {
                let next = groups.len() as i32;
                *groups.entry(group.clone()).or_insert(next)
            });
            let crafting_requirements = if properties.special {
                None
            } else {
                Some(
                    properties
                        .placement
                        .as_ref()
                        .map_or(&[][..], |placement| &placement.ingredients)
                        .iter()
                        .map(Ingredient::holder_set)
                        .collect(),
                )
            };
            let category = book.category.resolve(&recipe.recipe_type().key);
            for display in payload_displays {
                let id = RecipeDisplayId {
                    index: i32::try_from(displays.len()).expect("display id fits i32"),
                };
                let entry = RecipeDisplayEntry {
                    id,
                    display,
                    group,
                    category,
                    crafting_requirements: crafting_requirements.clone(),
                };
                recipe_to_display
                    .entry(recipe.key().clone())
                    .or_default()
                    .push(displays.len());
                displays.push(ServerDisplayInfo {
                    display: entry,
                    parent: recipe,
                });
            }
        }
        Self {
            displays,
            recipe_to_display,
        }
    }

    /// Resolves a server-assigned display id (vanilla
    /// `RecipeManager.getRecipeFromDisplay`): unknown or out-of-range ids
    /// resolve to nothing.
    #[must_use]
    pub fn get_recipe_from_display(&self, id: RecipeDisplayId) -> Option<&ServerDisplayInfo> {
        let index = usize::try_from(id.index).ok()?;
        self.displays.get(index)
    }

    /// Iterates the displays belonging to one recipe (vanilla
    /// `RecipeManager.listDisplaysForRecipe`); unknown keys produce nothing.
    pub fn list_displays_for_recipe(
        &self,
        key: &Identifier,
    ) -> impl Iterator<Item = &ServerDisplayInfo> + '_ {
        self.recipe_to_display
            .get(key)
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .filter_map(|index| self.displays.get(*index))
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&ServerDisplayInfo> {
        self.displays.get(index)
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.displays.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.displays.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ServerDisplayInfo> + '_ {
        self.displays.iter()
    }
}

/// Type-erased recipe reference returned by all-recipe and key lookup APIs.
#[derive(Clone, Copy)]
pub struct UntypedRecipeRef {
    pub(crate) recipe: &'static dyn ErasedRecipe,
}

impl UntypedRecipeRef {
    #[must_use]
    pub fn key(self) -> &'static Identifier {
        self.recipe.key()
    }

    #[must_use]
    pub fn recipe_type(self) -> RecipeTypeEntryRef {
        self.recipe.recipe_type()
    }

    #[must_use]
    pub fn data(self) -> &'static dyn RecipeData {
        self.recipe.data()
    }

    /// Recovers concrete data after an untyped key or all-recipe lookup.
    #[must_use]
    pub fn downcast_data<D: RecipeData + DowncastType>(self) -> Option<&'static D> {
        self.recipe.data().downcast_ref::<D>()
    }

    fn typed<D: RecipeMatches<I> + DowncastType, I: RecipeInput>(
        self,
        recipe_type: &'static RecipeType<D, I>,
    ) -> Option<TypedRecipeRef<D, I>> {
        if !std::ptr::eq(self.recipe.recipe_type(), recipe_type.entry()) {
            return None;
        }
        Some(TypedRecipeRef {
            key: self.recipe.key(),
            data: self.recipe.data().downcast_ref::<D>()?,
            recipe_type,
        })
    }
}

impl std::fmt::Debug for UntypedRecipeRef {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UntypedRecipeRef")
            .field("key", &self.recipe.key())
            .field("recipe_type", &self.recipe.recipe_type().key)
            .field("data", &self.recipe.data())
            .finish()
    }
}

/// Recipe reference with its concrete data and input types restored.
pub struct TypedRecipeRef<D: RecipeMatches<I> + DowncastType, I: RecipeInput> {
    key: &'static Identifier,
    data: &'static D,
    recipe_type: &'static RecipeType<D, I>,
}

impl<D: RecipeMatches<I> + DowncastType, I: RecipeInput> Copy for TypedRecipeRef<D, I> {}

impl<D: RecipeMatches<I> + DowncastType, I: RecipeInput> Clone for TypedRecipeRef<D, I> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: RecipeMatches<I> + DowncastType, I: RecipeInput> TypedRecipeRef<D, I> {
    #[must_use]
    pub const fn key(self) -> &'static Identifier {
        self.key
    }

    #[must_use]
    pub const fn data(self) -> &'static D {
        self.data
    }

    #[must_use]
    pub const fn recipe_type(self) -> &'static RecipeType<D, I> {
        self.recipe_type
    }
}

impl<D: RecipeMatches<I> + DowncastType, I: RecipeInput> std::fmt::Debug for TypedRecipeRef<D, I> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TypedRecipeRef")
            .field("key", &self.key)
            .field("recipe_type", &self.recipe_type.key())
            .field("data", &self.data)
            .finish()
    }
}

/// Caches the last successful recipe for one operational recipe type.
///
/// This mirrors Vanilla's `RecipeManager.CachedCheck`: the cached recipe is
/// tested first, then matching falls back to the registry's deterministic scan.
pub struct CachedRecipeCheck<D: RecipeMatches<I> + DowncastType, I: RecipeInput> {
    recipe_type: &'static RecipeType<D, I>,
    last_recipe: Option<TypedRecipeRef<D, I>>,
}

impl<D: RecipeMatches<I> + DowncastType, I: RecipeInput> CachedRecipeCheck<D, I> {
    /// Creates an empty cache for one recipe type.
    #[must_use]
    pub const fn new(recipe_type: &'static RecipeType<D, I>) -> Self {
        Self {
            recipe_type,
            last_recipe: None,
        }
    }

    /// Finds a matching recipe, testing the last successful recipe first.
    pub fn find_match(
        &mut self,
        registry: &RecipeRegistry,
        input: &I,
    ) -> Option<TypedRecipeRef<D, I>> {
        if input.is_empty() {
            return None;
        }
        if let Some(recipe) = self.last_recipe
            && recipe.data.matches(input)
        {
            return Some(recipe);
        }
        let recipe = registry.find_match(self.recipe_type, input)?;
        self.last_recipe = Some(recipe);
        Some(recipe)
    }
}

/// Typed recipes belonging to one operational recipe type.
pub struct TypedRecipeSet<'a, D: RecipeMatches<I> + DowncastType, I: RecipeInput> {
    registry: &'a RecipeRegistry,
    recipe_type: &'static RecipeType<D, I>,
    indices: &'a [usize],
}

impl<D: RecipeMatches<I> + DowncastType, I: RecipeInput> TypedRecipeSet<'_, D, I> {
    pub fn iter(&self) -> impl Iterator<Item = TypedRecipeRef<D, I>> + '_ {
        self.indices.iter().filter_map(|index| {
            self.registry
                .recipes
                .get(*index)
                .copied()
                .and_then(|recipe| recipe.typed(self.recipe_type))
        })
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.indices.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// Central storage for recipes of every registered concrete type.
pub struct RecipeRegistry {
    recipes: Vec<UntypedRecipeRef>,
    by_key: FxHashMap<Identifier, usize>,
    by_type: FxHashMap<Identifier, Vec<usize>>,
    allows_registering: bool,
}

impl RecipeRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            recipes: Vec::new(),
            by_key: FxHashMap::default(),
            by_type: FxHashMap::default(),
            allows_registering: true,
        }
    }

    /// Registers a concrete static recipe.
    pub fn register<D: RecipeMatches<I> + DowncastType, I: RecipeInput>(
        &mut self,
        recipe: &'static Recipe<D, I>,
    ) {
        assert!(
            self.allows_registering,
            "Cannot register recipes after the registry has been frozen"
        );
        assert!(
            !self.by_key.contains_key(recipe.key()),
            "Cannot register duplicate recipe key: {}",
            recipe.key()
        );
        let index = self.recipes.len();
        let erased = UntypedRecipeRef { recipe };
        self.recipes.push(erased);
        self.by_key.insert(recipe.key().clone(), index);
        self.by_type
            .entry(recipe.recipe_type().key().clone())
            .or_default()
            .push(index);
    }

    /// Replaces an entry with the same persistent key before freeze.
    pub fn replace<D: RecipeMatches<I> + DowncastType, I: RecipeInput>(
        &mut self,
        recipe: &'static Recipe<D, I>,
    ) -> Option<UntypedRecipeRef> {
        assert!(
            self.allows_registering,
            "Cannot replace recipes after the registry has been frozen"
        );
        let index = self.by_key.get(recipe.key()).copied()?;
        let replacement = UntypedRecipeRef { recipe };
        let previous = std::mem::replace(&mut self.recipes[index], replacement);
        if let Some(indices) = self.by_type.get_mut(&previous.recipe_type().key) {
            indices.retain(|stored| *stored != index);
        }
        self.by_type
            .entry(recipe.recipe_type().key().clone())
            .or_default()
            .push(index);
        Some(previous)
    }

    /// Freezes and sorts recipe lookup order by full identifier, matching
    /// Vanilla's sorted resource loading before `RecipeMap` construction.
    pub fn freeze(&mut self, recipe_types: &RecipeTypeRegistry) {
        for recipe in &self.recipes {
            let Some(registered_type) = recipe_types.by_key(&recipe.recipe_type().key) else {
                panic!(
                    "Recipe {} uses unregistered recipe type {}",
                    recipe.key(),
                    recipe.recipe_type().key
                );
            };
            assert!(
                std::ptr::eq(registered_type, recipe.recipe_type()),
                "Recipe {} does not use the canonical recipe type {}",
                recipe.key(),
                recipe.recipe_type().key
            );
            assert_eq!(
                recipe.data().downcast_type_key(),
                registered_type.data_type_key(),
                "Recipe {} data does not match recipe type {}",
                recipe.key(),
                recipe.recipe_type().key
            );
        }

        self.recipes
            .sort_by(|left, right| left.key().cmp(right.key()));
        self.by_key.clear();
        self.by_type.clear();
        for (index, recipe) in self.recipes.iter().copied().enumerate() {
            self.by_key.insert(recipe.key().clone(), index);
            self.by_type
                .entry(recipe.recipe_type().key.clone())
                .or_default()
                .push(index);
        }
        self.allows_registering = false;
    }

    #[must_use]
    pub fn by_key(&self, key: &Identifier) -> Option<UntypedRecipeRef> {
        self.by_key
            .get(key)
            .and_then(|index| self.recipes.get(*index))
            .copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = UntypedRecipeRef> + '_ {
        self.recipes.iter().copied()
    }

    #[must_use]
    pub fn by_type<D: RecipeMatches<I> + DowncastType, I: RecipeInput>(
        &self,
        recipe_type: &'static RecipeType<D, I>,
    ) -> TypedRecipeSet<'_, D, I> {
        TypedRecipeSet {
            registry: self,
            recipe_type,
            indices: self
                .by_type
                .get(recipe_type.key())
                .map_or(&[], Vec::as_slice),
        }
    }

    /// Iterates every matching recipe in deterministic key order.
    pub fn matching<'registry, D: RecipeMatches<I> + DowncastType, I: RecipeInput>(
        &'registry self,
        recipe_type: &'static RecipeType<D, I>,
        input: &'registry I,
    ) -> impl Iterator<Item = TypedRecipeRef<D, I>> + 'registry {
        let indices = self
            .by_type
            .get(recipe_type.key())
            .map_or(&[][..], Vec::as_slice);
        indices.iter().filter_map(move |index| {
            let recipe = self.recipes.get(*index).copied()?.typed(recipe_type)?;
            (!input.is_empty() && recipe.data.matches(input)).then_some(recipe)
        })
    }

    /// Finds the first matching recipe in deterministic registry order.
    #[must_use]
    pub fn find_match<D: RecipeMatches<I> + DowncastType, I: RecipeInput>(
        &self,
        recipe_type: &'static RecipeType<D, I>,
        input: &I,
    ) -> Option<TypedRecipeRef<D, I>> {
        if input.is_empty() {
            return None;
        }
        self.matching(recipe_type, input).next()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.recipes.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.recipes.is_empty()
    }
}

impl Default for RecipeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use vecta_utils::{DowncastType, DowncastTypeKey, Identifier};

    use crate::item_stack::ItemStack;
    use crate::item_stack_template::ItemStackTemplate;
    use crate::recipe::display::SlotDisplay;
    use crate::recipe::recipe_display::RecipeDisplay;
    use crate::recipe::registry::ServerDisplayInfo;
    use crate::recipe::{
        CachedRecipeCheck, CraftingBookCategory, CraftingInput, CraftingRecipe, DisplayIndex,
        Ingredient, PlacementInfo, Recipe, RecipeBookCategoryKind, RecipeBookProperties,
        RecipeData, RecipeDisplayId, RecipeInput, RecipeMatches, RecipeProperties, RecipeRegistry,
        RecipeType, RecipeTypeRegistry, ShapelessRecipe, vanilla_recipe_book_categories,
        vanilla_recipe_types,
    };
    use crate::{REGISTRY, init_vanilla_registry, vanilla_items, vanilla_recipes};

    #[test]
    fn generated_static_and_registry_lookup_share_the_same_recipe_data() {
        init_vanilla_registry();

        let direct = &*vanilla_recipes::IRON_PICKAXE;
        let Some(registered) = REGISTRY.recipes.by_key(direct.key()) else {
            panic!("generated iron pickaxe recipe was not registered");
        };
        let Some(registered_data) = registered.downcast_data::<CraftingRecipe>() else {
            panic!("registered iron pickaxe did not retain crafting data");
        };

        assert!(std::ptr::eq(registered_data, direct.data()));
        let CraftingRecipe::Shaped(shaped) = direct.data() else {
            panic!("iron pickaxe must remain a shaped recipe");
        };
        assert_eq!((shaped.width, shaped.height), (3, 3));
    }

    #[test]
    fn generated_recipe_result_preserves_extracted_component_patch() {
        use crate::data_components::vanilla_components::SUSPICIOUS_STEW_EFFECTS;

        init_vanilla_registry();
        let CraftingRecipe::Shapeless(recipe) = vanilla_recipes::SUSPICIOUS_STEW_FROM_ALLIUM.data()
        else {
            panic!("allium suspicious stew must remain shapeless");
        };

        let result = recipe.result.create();
        let Some(effects) = result.get(SUSPICIOUS_STEW_EFFECTS) else {
            panic!("suspicious stew result lost its extracted effects");
        };
        assert_eq!(effects.effects().len(), 1);
        assert_eq!(effects.effects()[0].duration(), 60);
    }

    #[test]
    fn typed_matching_finds_the_generated_iron_pickaxe_recipe() {
        init_vanilla_registry();
        let empty = ItemStack::empty;
        let input = CraftingInput::new(
            3,
            3,
            vec![
                ItemStack::new(&vanilla_items::IRON_INGOT),
                ItemStack::new(&vanilla_items::IRON_INGOT),
                ItemStack::new(&vanilla_items::IRON_INGOT),
                empty(),
                ItemStack::new(&vanilla_items::STICK),
                empty(),
                empty(),
                ItemStack::new(&vanilla_items::STICK),
                empty(),
            ],
        );

        let Some(found) = REGISTRY
            .recipes
            .find_match(&vanilla_recipe_types::CRAFTING, &input)
        else {
            panic!("iron pickaxe input should match a crafting recipe");
        };

        assert_eq!(found.key(), vanilla_recipes::IRON_PICKAXE.key());
    }

    #[test]
    fn every_extracted_recipe_is_present_in_its_operational_type_bucket() {
        init_vanilla_registry();

        assert_eq!(REGISTRY.recipe_types.len(), 7);
        assert_eq!(REGISTRY.recipes.len(), 1_763);
        assert_eq!(
            REGISTRY
                .recipes
                .by_type(&vanilla_recipe_types::CRAFTING)
                .len(),
            1_266
        );
        assert_eq!(
            REGISTRY
                .recipes
                .by_type(&vanilla_recipe_types::SMELTING)
                .len(),
            73
        );
        assert_eq!(
            REGISTRY
                .recipes
                .by_type(&vanilla_recipe_types::BLASTING)
                .len(),
            25
        );
        assert_eq!(
            REGISTRY
                .recipes
                .by_type(&vanilla_recipe_types::SMOKING)
                .len(),
            9
        );
        assert_eq!(
            REGISTRY
                .recipes
                .by_type(&vanilla_recipe_types::CAMPFIRE_COOKING)
                .len(),
            9
        );
        assert_eq!(
            REGISTRY
                .recipes
                .by_type(&vanilla_recipe_types::STONECUTTING)
                .len(),
            351
        );
        assert_eq!(
            REGISTRY
                .recipes
                .by_type(&vanilla_recipe_types::SMITHING)
                .len(),
            30
        );
    }

    #[derive(Debug)]
    struct PluginData {
        required: i32,
    }

    // SAFETY: This test-only key uniquely identifies the plugin-like recipe data.
    unsafe impl DowncastType for PluginData {
        const TYPE_KEY: DowncastTypeKey =
            DowncastTypeKey::new("vecta:test/recipe_data/plugin_machine");
    }

    impl RecipeData for PluginData {}

    #[derive(Debug)]
    struct PluginInput(i32);

    // SAFETY: This test-only key uniquely identifies the plugin-like input snapshot.
    unsafe impl DowncastType for PluginInput {
        const TYPE_KEY: DowncastTypeKey =
            DowncastTypeKey::new("vecta:test/recipe_input/plugin_machine");
    }

    impl RecipeInput for PluginInput {
        fn is_empty(&self) -> bool {
            false
        }
    }

    impl RecipeMatches<PluginInput> for PluginData {
        fn matches(&self, input: &PluginInput) -> bool {
            self.required == input.0
        }
    }

    static PLUGIN_TYPE: RecipeType<PluginData, PluginInput> =
        RecipeType::new(Identifier::new_static("test_plugin", "pulverizing"));
    static PLUGIN_RECIPE: LazyLock<Recipe<PluginData, PluginInput>> = LazyLock::new(|| {
        Recipe::new(
            Identifier::new_static("test_plugin", "pulverize_ore"),
            &PLUGIN_TYPE,
            PluginData { required: 7 },
        )
    });

    #[test]
    fn plugin_type_keeps_custom_data_typed_through_matching_and_key_lookup() {
        let mut types = RecipeTypeRegistry::new();
        types.register(&PLUGIN_TYPE);
        types.freeze();
        let mut recipes = RecipeRegistry::new();
        recipes.register(&PLUGIN_RECIPE);
        recipes.freeze(&types);

        let Some(found) = recipes.find_match(&PLUGIN_TYPE, &PluginInput(7)) else {
            panic!("plugin recipe should match its custom input");
        };
        assert_eq!(found.data().required, 7);

        let Some(untyped) = recipes.by_key(PLUGIN_RECIPE.key()) else {
            panic!("plugin recipe should be available by persistent key");
        };
        assert_eq!(
            untyped
                .downcast_data::<PluginData>()
                .map(|data| data.required),
            Some(7)
        );
    }

    static CACHE_MATCH_CALLS: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug)]
    struct CacheData {
        required: i32,
    }

    // SAFETY: This test-only key uniquely identifies cached recipe data.
    unsafe impl DowncastType for CacheData {
        const TYPE_KEY: DowncastTypeKey =
            DowncastTypeKey::new("vecta:test/recipe_data/cached_check");
    }

    impl RecipeData for CacheData {}

    #[derive(Debug)]
    struct CacheInput(i32);

    // SAFETY: This test-only key uniquely identifies cached recipe input.
    unsafe impl DowncastType for CacheInput {
        const TYPE_KEY: DowncastTypeKey =
            DowncastTypeKey::new("vecta:test/recipe_input/cached_check");
    }

    impl RecipeInput for CacheInput {
        fn is_empty(&self) -> bool {
            false
        }
    }

    impl RecipeMatches<CacheInput> for CacheData {
        fn matches(&self, input: &CacheInput) -> bool {
            CACHE_MATCH_CALLS.fetch_add(1, Ordering::Relaxed);
            self.required == input.0
        }
    }

    static CACHE_TYPE: RecipeType<CacheData, CacheInput> =
        RecipeType::new(Identifier::new_static("test_plugin", "cached_machine"));
    static CACHE_NON_MATCH: LazyLock<Recipe<CacheData, CacheInput>> = LazyLock::new(|| {
        Recipe::new(
            Identifier::new_static("test_plugin", "a_non_match"),
            &CACHE_TYPE,
            CacheData { required: 3 },
        )
    });
    static CACHE_MATCH: LazyLock<Recipe<CacheData, CacheInput>> = LazyLock::new(|| {
        Recipe::new(
            Identifier::new_static("test_plugin", "b_match"),
            &CACHE_TYPE,
            CacheData { required: 7 },
        )
    });

    #[test]
    fn cached_check_tests_the_last_successful_recipe_before_scanning() {
        let mut types = RecipeTypeRegistry::new();
        types.register(&CACHE_TYPE);
        types.freeze();
        let mut recipes = RecipeRegistry::new();
        recipes.register(&CACHE_NON_MATCH);
        recipes.register(&CACHE_MATCH);
        recipes.freeze(&types);
        let mut cache = CachedRecipeCheck::new(&CACHE_TYPE);
        CACHE_MATCH_CALLS.store(0, Ordering::Relaxed);

        assert_eq!(
            cache
                .find_match(&recipes, &CacheInput(7))
                .map(super::TypedRecipeRef::key),
            Some(CACHE_MATCH.key())
        );
        assert_eq!(CACHE_MATCH_CALLS.load(Ordering::Relaxed), 2);

        assert_eq!(
            cache
                .find_match(&recipes, &CacheInput(7))
                .map(super::TypedRecipeRef::key),
            Some(CACHE_MATCH.key())
        );
        assert_eq!(CACHE_MATCH_CALLS.load(Ordering::Relaxed), 3);

        assert!(cache.find_match(&recipes, &CacheInput(9)).is_none());
        assert_eq!(CACHE_MATCH_CALLS.load(Ordering::Relaxed), 6);
        assert_eq!(
            cache
                .find_match(&recipes, &CacheInput(7))
                .map(super::TypedRecipeRef::key),
            Some(CACHE_MATCH.key())
        );
        assert_eq!(CACHE_MATCH_CALLS.load(Ordering::Relaxed), 7);
    }

    #[test]
    fn shapeless_matching_backtracks_when_ingredients_overlap() {
        init_vanilla_registry();
        let choice = Ingredient::Choice(Box::leak(
            vec![&*vanilla_items::RED_DYE, &*vanilla_items::BLUE_DYE].into_boxed_slice(),
        ));
        let exact_red = Ingredient::Item(&vanilla_items::RED_DYE);
        let input = CraftingInput::new(
            2,
            1,
            vec![
                ItemStack::new(&vanilla_items::RED_DYE),
                ItemStack::new(&vanilla_items::BLUE_DYE),
            ],
        );
        let recipe = CraftingRecipe::Shapeless(crate::recipe::ShapelessRecipe::new(
            crate::recipe::RecipeProperties::special(),
            vec![choice, exact_red].into_boxed_slice(),
            crate::item_stack_template::ItemStackTemplate::new(&vanilla_items::PURPLE_DYE),
        ));

        assert!(recipe.matches(&input));
    }

    #[test]
    fn display_index_assigns_sequential_ids_in_registration_order() {
        init_vanilla_registry();
        let index = DisplayIndex::build(&REGISTRY.recipes);
        assert!(
            index.len() > 1_000,
            "vanilla recipes must unpack more than 1000 displays, got {}",
            index.len()
        );
        for (position, info) in index.iter().enumerate() {
            assert_eq!(
                info.display.id,
                RecipeDisplayId {
                    index: i32::try_from(position).expect("position fits i32"),
                },
                "display ids must be the entry position"
            );
        }
        assert!(
            index
                .get_recipe_from_display(RecipeDisplayId { index: -1 })
                .is_none(),
            "negative display ids resolve to nothing"
        );
        assert!(
            index
                .get_recipe_from_display(RecipeDisplayId {
                    index: index.len() as i32,
                })
                .is_none(),
            "out-of-range display ids resolve to nothing"
        );
        let first = index
            .get_recipe_from_display(RecipeDisplayId { index: 0 })
            .expect("first display exists");
        assert_eq!(first.display.id.index, 0);
    }

    #[test]
    fn iron_pickaxe_display_links_to_its_recipe_with_the_vanilla_shape() {
        init_vanilla_registry();
        let index = DisplayIndex::build(&REGISTRY.recipes);
        let entries: Vec<&ServerDisplayInfo> = index
            .list_displays_for_recipe(vanilla_recipes::IRON_PICKAXE.key())
            .collect();
        assert_eq!(entries.len(), 1, "shaped recipes unpack one display");
        let entry = &entries[0].display;
        let RecipeDisplay::Shaped {
            width,
            height,
            ingredients,
            result,
            crafting_station,
        } = &entry.display
        else {
            panic!("iron pickaxe must unpack a shaped display");
        };
        assert_eq!((*width, *height), (3, 3));
        assert_eq!(ingredients.len(), 9);
        let shaped = match vanilla_recipes::IRON_PICKAXE.data() {
            CraftingRecipe::Shaped(shaped) => shaped,
            other => panic!("iron pickaxe must keep shaped data, got {other:?}"),
        };
        assert_eq!(
            *ingredients,
            shaped
                .pattern
                .iter()
                .map(Ingredient::display)
                .collect::<Vec<_>>()
        );
        assert_eq!(*result, SlotDisplay::ItemStack(shaped.result.clone()));
        assert_eq!(
            *crafting_station,
            SlotDisplay::Item(&vanilla_items::CRAFTING_TABLE)
        );
        assert_eq!(
            entry.category,
            &vanilla_recipe_book_categories::CRAFTING_EQUIPMENT
        );
        // Not special: the placement ingredients are the stacked requirements.
        let Some(requirements) = &entry.crafting_requirements else {
            panic!("regular recipes carry crafting requirements");
        };
        let placement = shaped
            .properties
            .placement
            .as_ref()
            .expect("shaped placement");
        assert_eq!(
            requirements,
            &placement
                .ingredients
                .iter()
                .map(Ingredient::holder_set)
                .collect::<Vec<_>>()
        );
        assert!(!placement.ingredients.is_empty());
        assert_eq!(entries[0].parent.key(), vanilla_recipes::IRON_PICKAXE.key());
    }

    #[test]
    fn recipes_sharing_a_group_share_one_group_id_per_unpack() {
        init_vanilla_registry();
        let index = DisplayIndex::build(&REGISTRY.recipes);
        let oak = index
            .list_displays_for_recipe(vanilla_recipes::OAK_BOAT.key())
            .next()
            .expect("oak boat unpacks displays")
            .display
            .group;
        let acacia = index
            .list_displays_for_recipe(vanilla_recipes::ACACIA_BOAT.key())
            .next()
            .expect("acacia boat unpacks displays")
            .display
            .group;
        assert!(oak.is_some(), "boat recipes declare a book group");
        assert_eq!(oak, acacia, "recipes sharing a group share one group id");
    }

    #[test]
    fn baked_potato_display_carries_the_furnace_station_and_values() {
        init_vanilla_registry();
        let index = DisplayIndex::build(&REGISTRY.recipes);
        let entries: Vec<&ServerDisplayInfo> = index
            .list_displays_for_recipe(vanilla_recipes::BAKED_POTATO.key())
            .collect();
        assert_eq!(entries.len(), 1, "cooking recipes unpack one display");
        let entry = &entries[0].display;
        let RecipeDisplay::Furnace {
            ingredient,
            fuel,
            result,
            crafting_station,
            duration,
            experience,
        } = &entry.display
        else {
            panic!("baked potato must unpack a furnace display");
        };
        let cooking = vanilla_recipes::BAKED_POTATO.data();
        assert_eq!(*ingredient, cooking.ingredient.display());
        assert_eq!(*fuel, SlotDisplay::AnyFuel);
        assert_eq!(*result, SlotDisplay::ItemStack(cooking.result.clone()));
        assert_eq!(
            *crafting_station,
            SlotDisplay::Item(&vanilla_items::FURNACE)
        );
        assert_eq!(*duration, cooking.cooking_time);
        assert_eq!(*experience, cooking.experience);
        assert_eq!(
            entry.category,
            &vanilla_recipe_book_categories::FURNACE_FOOD
        );
    }

    #[test]
    fn smithing_trim_recipes_render_the_trim_demo_result() {
        init_vanilla_registry();
        let index = DisplayIndex::build(&REGISTRY.recipes);
        let entries: Vec<&ServerDisplayInfo> = index
            .list_displays_for_recipe(
                vanilla_recipes::BOLT_ARMOR_TRIM_SMITHING_TEMPLATE_SMITHING_TRIM.key(),
            )
            .collect();
        assert_eq!(entries.len(), 1, "smithing trim recipes unpack one display");
        let RecipeDisplay::Smithing {
            result,
            crafting_station,
            ..
        } = &entries[0].display.display
        else {
            panic!("smithing trim must unpack a smithing display");
        };
        let SlotDisplay::SmithingTrim { base, material, .. } = result else {
            panic!("trim results use the smithing trim demo display");
        };
        assert!(
            matches!(**base, SlotDisplay::Tag(_)),
            "trim base is an ingredient tag display"
        );
        assert!(
            matches!(**material, SlotDisplay::Tag(_)),
            "trim material is an ingredient tag display"
        );
        assert_eq!(
            *crafting_station,
            SlotDisplay::Item(&vanilla_items::SMITHING_TABLE)
        );
        assert_eq!(
            entries[0].display.category,
            &vanilla_recipe_book_categories::SMITHING
        );
    }

    #[test]
    fn special_recipes_drop_crafting_requirements() {
        static SPECIAL: LazyLock<Recipe<CraftingRecipe, CraftingInput>> = LazyLock::new(|| {
            Recipe::new(
                Identifier::new_static("test_plugin", "a_special_display"),
                &vanilla_recipe_types::CRAFTING,
                CraftingRecipe::Shapeless(ShapelessRecipe::new(
                    RecipeProperties {
                        special: true,
                        placement: Some(PlacementInfo::from_ingredients(&[Ingredient::Item(
                            &vanilla_items::STICK,
                        )])),
                        recipe_book: Some(RecipeBookProperties {
                            category: RecipeBookCategoryKind::Crafting(CraftingBookCategory::Misc),
                            group: None,
                            show_notification: true,
                        }),
                    },
                    vec![Ingredient::Item(&vanilla_items::STICK)].into_boxed_slice(),
                    ItemStackTemplate::new(&vanilla_items::STICK),
                )),
            )
        });

        let mut types = RecipeTypeRegistry::new();
        types.register(&vanilla_recipe_types::CRAFTING);
        types.freeze();
        let mut recipes = RecipeRegistry::new();
        recipes.register(&SPECIAL);
        recipes.freeze(&types);

        let index = DisplayIndex::build(&recipes);
        let entry = index
            .list_displays_for_recipe(SPECIAL.key())
            .next()
            .expect("special shapeless recipes still unpack a display");
        assert!(
            entry.display.crafting_requirements.is_none(),
            "special recipes carry no crafting requirements"
        );
    }
}
