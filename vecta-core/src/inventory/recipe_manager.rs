//! Recipe matching and crafting-grid interpretation.

use vecta_registry::data_components::components::ArmorTrim;
use vecta_registry::data_components::vanilla_components::{
    BANNER_PATTERNS, BASE_COLOR, DYE, DYED_COLOR, ENCHANTMENTS, FIREWORK_EXPLOSION, FIREWORKS,
    MAP_POST_PROCESSING, MAX_DAMAGE, POT_DECORATIONS, POTION_CONTENTS, PROVIDES_TRIM_MATERIAL,
    TRIM, WRITTEN_BOOK_CONTENT,
};
use vecta_registry::data_components::vanilla_components::{
    DyedItemColor, FireworkExplosion, FireworkExplosionShape, Fireworks, ItemEnchantments,
    MapPostProcessing, PotDecorations,
};
use vecta_registry::items::ItemRef;
use vecta_registry::recipe::{
    BookCloningRecipe, CookingRecipe, CraftingInput, CraftingRecipe, DyeRecipe,
    FireworkRocketRecipe, FireworkStarFadeRecipe, FireworkStarRecipe, Ingredient,
    PositionedCraftingInput, RecipePropertySet, RecipePropertySetKey, ShieldDecorationRecipe,
    SingleInputEntry, SingleItemRecipeInput, SlotDisplay, SmithingRecipe, SmithingRecipeInput,
    StonecuttingRecipe, TypedRecipeRef, vanilla_recipe_types,
};
use vecta_registry::trim_pattern::TrimPatternRef;
use vecta_registry::{
    DyeColor, ItemStackTemplate, REGISTRY, RegistryExt as _, RegistryHolder,
    TaggedRegistryExt as _, item_stack::ItemStack, vanilla_enchantment_tags, vanilla_items,
};
use vecta_utils::Identifier;

use crate::inventory::container::CraftingContainer;

use super::container::Container;

/// Typed reference to one registered crafting recipe.
pub type CraftingRecipeRef = TypedRecipeRef<CraftingRecipe, CraftingInput>;

/// Typed reference to one registered stonecutting recipe.
pub type StonecuttingRecipeRef = TypedRecipeRef<StonecuttingRecipe, SingleItemRecipeInput>;

/// Typed reference to one registered smithing recipe.
pub type SmithingRecipeRef = TypedRecipeRef<SmithingRecipe, SmithingRecipeInput>;

/// Finds the first matching smithing recipe in deterministic registry order.
#[must_use]
pub fn find_smithing_recipe(input: &SmithingRecipeInput) -> Option<SmithingRecipeRef> {
    REGISTRY
        .recipes
        .find_match(&vanilla_recipe_types::SMITHING, input)
}

/// Interprets passive vanilla smithing data for a concrete three-slot input.
///
/// Mirrors `SmithingTransformRecipe.assemble` (the result template with the
/// base item's component patch) and `SmithingTrimRecipe.assemble` (the base
/// item with the addition's trim material and the recipe's pattern).
#[must_use]
pub fn assemble_smithing(recipe: SmithingRecipeRef, input: &SmithingRecipeInput) -> ItemStack {
    match recipe.data() {
        SmithingRecipe::Transform(recipe) => recipe
            .result
            .apply(recipe.result.count(), input.base.components_patch()),
        SmithingRecipe::Trim(recipe) => apply_trim(&input.base, &input.addition, recipe.pattern),
    }
}

/// Mirrors `SmithingTrimRecipe.applyTrim`.
fn apply_trim(base: &ItemStack, material_item: &ItemStack, pattern: TrimPatternRef) -> ItemStack {
    let Some(material) = material_item.get(PROVIDES_TRIM_MATERIAL) else {
        return ItemStack::empty();
    };
    let new_trim = ArmorTrim::new(
        material.material().clone(),
        RegistryHolder::reference(pattern),
    );
    if base.get(TRIM) == Some(&new_trim) {
        return ItemStack::empty();
    }

    let mut trimmed = base.copy_with_count(1);
    trimmed.set(TRIM, new_trim);
    trimmed
}

/// Items that at least one smithing recipe accepts in each input role.
///
/// Vanilla derives `RecipePropertySet.SMITHING_TEMPLATE` / `SMITHING_BASE` /
/// `SMITHING_ADDITION` from every loaded recipe in `finalizeRecipeLoading` and
/// uses them as the three `ItemCombinerMenuSlotDefinition` placement
/// predicates. Vecta has no property-set registry, so the same sets are derived
/// here from the registered recipes.
pub struct SmithingItemTests {
    template: Vec<ItemRef>,
    base: Vec<ItemRef>,
    addition: Vec<ItemRef>,
}

impl SmithingItemTests {
    /// Collects the accepted items of each role across every smithing recipe.
    #[must_use]
    pub fn collect() -> Self {
        let mut tests = Self {
            template: Vec::new(),
            base: Vec::new(),
            addition: Vec::new(),
        };
        for recipe in REGISTRY
            .recipes
            .by_type(&vanilla_recipe_types::SMITHING)
            .iter()
        {
            let (template, base, addition) = match recipe.data() {
                SmithingRecipe::Transform(recipe) => (
                    recipe.template.as_ref(),
                    Some(&recipe.base),
                    recipe.addition.as_ref(),
                ),
                SmithingRecipe::Trim(recipe) => (
                    Some(&recipe.template),
                    Some(&recipe.base),
                    Some(&recipe.addition),
                ),
            };
            extend_items(&mut tests.template, template);
            extend_items(&mut tests.base, base);
            extend_items(&mut tests.addition, addition);
        }
        tests
    }

    /// Whether `stack` may be placed in the template slot.
    #[must_use]
    pub fn accepts_template(&self, stack: &ItemStack) -> bool {
        accepts(&self.template, stack)
    }

    /// Whether `stack` may be placed in the base slot.
    #[must_use]
    pub fn accepts_base(&self, stack: &ItemStack) -> bool {
        accepts(&self.base, stack)
    }

    /// Whether `stack` may be placed in the addition slot.
    #[must_use]
    pub fn accepts_addition(&self, stack: &ItemStack) -> bool {
        accepts(&self.addition, stack)
    }
}

fn accepts(items: &[ItemRef], stack: &ItemStack) -> bool {
    !stack.is_empty() && items.contains(&stack.item)
}

fn extend_items(target: &mut Vec<ItemRef>, ingredient: Option<&Ingredient>) {
    let Some(ingredient) = ingredient else {
        return;
    };
    for item in ingredient.get_items() {
        if !target.contains(&item) {
            target.push(item);
        }
    }
}

/// Updates a result container from the first matching crafting recipe.
pub fn slot_changed_crafting_grid<R: Container>(crafting: &CraftingContainer, result: &mut R) {
    result.set_item(
        0,
        assemble_for_container(crafting).unwrap_or_else(ItemStack::empty),
    );
}

/// Finds the first crafting recipe in deterministic registry order.
#[must_use]
pub fn find_recipe(crafting: &CraftingContainer) -> Option<CraftingRecipeRef> {
    let positioned = crafting.as_positioned_input();
    REGISTRY
        .recipes
        .find_match(&vanilla_recipe_types::CRAFTING, &positioned.input)
}

/// Finds and interprets the output for the current crafting container.
#[must_use]
pub fn assemble_for_container(crafting: &CraftingContainer) -> Option<ItemStack> {
    let positioned = crafting.as_positioned_input();
    let recipe = find_recipe(crafting)?;
    Some(assemble_recipe(recipe, &positioned.input))
}

/// Interprets passive vanilla crafting data for a concrete input snapshot.
#[must_use]
pub fn assemble_recipe(recipe: CraftingRecipeRef, input: &CraftingInput) -> ItemStack {
    match recipe.data() {
        CraftingRecipe::Shaped(recipe) => recipe.result.create(),
        CraftingRecipe::Shapeless(recipe) => recipe.result.create(),
        CraftingRecipe::Transmute(recipe) => {
            let material_count = input
                .items
                .iter()
                .filter(|stack| !stack.is_empty() && recipe.material.test(stack))
                .count();
            let extra_count = if recipe.add_material_count_to_result {
                i32::try_from(material_count).unwrap_or(i32::MAX)
            } else {
                0
            };
            input
                .items
                .iter()
                .find(|stack| !stack.is_empty() && recipe.input.test(stack))
                .map_or_else(ItemStack::empty, |source| match &recipe.result {
                    Some(result) => result.apply(
                        result.count().saturating_add(extra_count),
                        source.components_patch(),
                    ),
                    None => source.copy_with_count(1i32.saturating_add(extra_count)),
                })
        }
        CraftingRecipe::Dye(recipe) => assemble_dye(recipe, input),
        CraftingRecipe::DecoratedPot(recipe) => {
            // Vanilla stores one decoration item per side, never the slot's count.
            let decoration = |x: usize, y: usize| {
                ItemStackTemplate::from_stack(&input.get(x, y).copy_with_count(1)).ok()
            };
            let decorations = PotDecorations::new(
                decoration(1, 0),
                decoration(0, 1),
                decoration(2, 1),
                decoration(1, 2),
            );
            let mut result = recipe.result.create();
            result.set(POT_DECORATIONS, decorations);
            result
        }
        CraftingRecipe::Imbue(recipe) => {
            let mut result = recipe.result.create();
            if let Some(contents) = input.get(1, 1).get(POTION_CONTENTS) {
                result.set(POTION_CONTENTS, contents.clone());
            }
            result
        }
        CraftingRecipe::BannerDuplicate(recipe) => input
            .items
            .iter()
            .find(|stack| {
                recipe.banner.test(stack)
                    && stack
                        .get(BANNER_PATTERNS)
                        .is_some_and(|patterns| !patterns.layers().is_empty())
            })
            .map_or_else(ItemStack::empty, |source| {
                recipe
                    .result
                    .apply(recipe.result.count(), source.components_patch())
            }),
        CraftingRecipe::BookCloning(recipe) => assemble_book_cloning(recipe, input),
        CraftingRecipe::FireworkRocket(recipe) => assemble_firework_rocket(recipe, input),
        CraftingRecipe::FireworkStar(recipe) => assemble_firework_star(recipe, input),
        CraftingRecipe::FireworkStarFade(recipe) => assemble_firework_fade(recipe, input),
        CraftingRecipe::MapExtending(recipe) => {
            let source = input.get(1, 1);
            let mut result = match &recipe.result {
                Some(result) => result.apply(result.count(), source.components_patch()),
                None => source.copy_with_count(1),
            };
            result.set(MAP_POST_PROCESSING, MapPostProcessing::Scale);
            result
        }
        CraftingRecipe::RepairItem(_) => assemble_repair(input),
        CraftingRecipe::ShieldDecoration(recipe) => assemble_shield(recipe, input),
    }
}

/// Returns crafting remainders and the positioned input used to map them back
/// to the original grid.
#[must_use]
pub fn get_remaining_items(
    crafting: &CraftingContainer,
) -> Option<(Vec<ItemStack>, PositionedCraftingInput)> {
    let positioned = crafting.as_positioned_input();
    let recipe = find_recipe(crafting)?;
    let mut remainders: Vec<_> = positioned
        .input
        .items
        .iter()
        .map(|stack| {
            if stack.is_empty() {
                ItemStack::empty()
            } else {
                stack.item().get_crafting_remainder()
            }
        })
        .collect();

    match recipe.data() {
        CraftingRecipe::BannerDuplicate(_) => {
            for (slot, stack) in positioned.input.items.iter().enumerate() {
                if remainders[slot].is_empty()
                    && stack
                        .get(BANNER_PATTERNS)
                        .is_some_and(|patterns| !patterns.layers().is_empty())
                {
                    remainders[slot] = stack.copy_with_count(1);
                }
            }
        }
        CraftingRecipe::BookCloning(_) => {
            for (slot, stack) in positioned.input.items.iter().enumerate() {
                if remainders[slot].is_empty() && stack.has(WRITTEN_BOOK_CONTENT) {
                    remainders[slot] = stack.copy_with_count(1);
                    break;
                }
            }
        }
        _ => {}
    }

    Some((remainders, positioned))
}

fn assemble_dye(recipe: &DyeRecipe, input: &CraftingInput) -> ItemStack {
    let mut target = None;
    let mut dyes = Vec::new();
    for stack in input.items.iter().filter(|stack| !stack.is_empty()) {
        if recipe.target.test(stack) {
            target = Some(stack);
        } else if recipe.dye.test(stack) {
            dyes.push(stack.get(DYE).copied().unwrap_or(DyeColor::White));
        }
    }
    let Some(target) = target else {
        return ItemStack::empty();
    };
    if dyes.is_empty() {
        return ItemStack::empty();
    }
    let color = DyedItemColor::apply_dyes(target.get(DYED_COLOR).copied(), &dyes);
    let mut result = recipe
        .result
        .apply(recipe.result.count(), target.components_patch());
    result.set(DYED_COLOR, color);
    result
}

fn assemble_book_cloning(recipe: &BookCloningRecipe, input: &CraftingInput) -> ItemStack {
    let Some(source) = input
        .items
        .iter()
        .find(|stack| !stack.is_empty() && recipe.source.test(stack))
    else {
        return ItemStack::empty();
    };
    let Some(content) = source.get(WRITTEN_BOOK_CONTENT) else {
        return ItemStack::empty();
    };
    let material_count = input
        .items
        .iter()
        .filter(|stack| !stack.is_empty() && recipe.material.test(stack))
        .count();
    let Ok(extra_count) = i32::try_from(material_count.saturating_sub(1)) else {
        return ItemStack::empty();
    };
    let mut result = recipe.result.apply(
        recipe.result.count().saturating_add(extra_count),
        source.components_patch(),
    );
    result.set(WRITTEN_BOOK_CONTENT, content.craft_copy());
    result
}

fn assemble_firework_rocket(recipe: &FireworkRocketRecipe, input: &CraftingInput) -> ItemStack {
    let mut flight_duration = 0;
    let mut explosions = Vec::new();
    for stack in input.items.iter().filter(|stack| !stack.is_empty()) {
        if recipe.fuel.test(stack) {
            flight_duration += 1;
        } else if recipe.star.test(stack)
            && let Some(explosion) = stack.get(FIREWORK_EXPLOSION)
        {
            explosions.push(explosion.clone());
        }
    }
    let Ok(fireworks) = Fireworks::new(flight_duration, explosions) else {
        return ItemStack::empty();
    };
    let mut result = recipe.result.create();
    result.set(FIREWORKS, fireworks);
    result
}

fn assemble_firework_star(recipe: &FireworkStarRecipe, input: &CraftingInput) -> ItemStack {
    let mut shape = FireworkExplosionShape::SmallBall;
    let mut has_twinkle = false;
    let mut has_trail = false;
    let mut colors = Vec::new();
    for stack in input.items.iter().filter(|stack| !stack.is_empty()) {
        if let Some((found_shape, _)) = recipe
            .shapes
            .iter()
            .find(|(_, ingredient)| ingredient.test(stack))
        {
            shape = *found_shape;
        } else if recipe.twinkle.test(stack) {
            has_twinkle = true;
        } else if recipe.trail.test(stack) {
            has_trail = true;
        } else if recipe.dye.test(stack) {
            colors.push(
                stack
                    .get(DYE)
                    .copied()
                    .unwrap_or(DyeColor::White)
                    .firework_color(),
            );
        }
    }
    let mut result = recipe.result.create();
    result.set(
        FIREWORK_EXPLOSION,
        FireworkExplosion::new(shape, colors, Vec::new(), has_trail, has_twinkle),
    );
    result
}

fn assemble_firework_fade(recipe: &FireworkStarFadeRecipe, input: &CraftingInput) -> ItemStack {
    let Some(target) = input
        .items
        .iter()
        .find(|stack| !stack.is_empty() && recipe.target.test(stack))
    else {
        return ItemStack::empty();
    };
    let fade_colors: Vec<_> = input
        .items
        .iter()
        .filter(|stack| !stack.is_empty() && recipe.dye.test(stack))
        .map(|stack| {
            stack
                .get(DYE)
                .copied()
                .unwrap_or(DyeColor::White)
                .firework_color()
        })
        .collect();
    let mut result = recipe
        .result
        .apply(recipe.result.count(), target.components_patch());
    let explosion = target
        .get(FIREWORK_EXPLOSION)
        .cloned()
        .unwrap_or_default()
        .with_fade_colors(fade_colors);
    result.set(FIREWORK_EXPLOSION, explosion);
    result
}

fn assemble_repair(input: &CraftingInput) -> ItemStack {
    let mut inputs = input.items.iter().filter(|stack| !stack.is_empty());
    let Some(first) = inputs.next() else {
        return ItemStack::empty();
    };
    let Some(second) = inputs.next() else {
        return ItemStack::empty();
    };
    let durability = first.get_max_damage().max(second.get_max_damage());
    let remaining = (first.get_max_damage() - first.get_damage_value())
        + (second.get_max_damage() - second.get_damage_value())
        + durability * 5 / 100;
    let mut result = ItemStack::new(first.item());
    result.set(MAX_DAMAGE, durability);
    result.set_damage_value((durability - remaining).max(0));

    let mut curses = ItemEnchantments::empty();
    for enchantments in [
        first.get_enchantments_for_crafting(),
        second.get_enchantments_for_crafting(),
    ]
    .into_iter()
    .flatten()
    {
        for (key, level) in enchantments.iter() {
            let Some(enchantment) = REGISTRY.enchantments.by_key(key) else {
                continue;
            };
            if REGISTRY.enchantments.is_in_tag(
                enchantment,
                &vanilla_enchantment_tags::EnchantmentTag::CURSE,
            ) {
                curses.upgrade(key.clone(), *level);
            }
        }
    }
    result.set(ENCHANTMENTS, curses);
    result
}

fn assemble_shield(recipe: &ShieldDecorationRecipe, input: &CraftingInput) -> ItemStack {
    let Some(banner) = input
        .items
        .iter()
        .find(|stack| !stack.is_empty() && recipe.banner.test(stack))
    else {
        return ItemStack::empty();
    };
    let Some(target) = input
        .items
        .iter()
        .find(|stack| !stack.is_empty() && recipe.target.test(stack))
    else {
        return ItemStack::empty();
    };
    let mut result = recipe
        .result
        .apply(recipe.result.count(), target.components_patch());
    if let Some(patterns) = banner.get(BANNER_PATTERNS) {
        result.set(BANNER_PATTERNS, patterns.clone());
    }
    result.set(BASE_COLOR, banner_color(banner));
    result
}

fn banner_color(stack: &ItemStack) -> DyeColor {
    let banners = [
        (&*vanilla_items::WHITE_BANNER, DyeColor::White),
        (&*vanilla_items::ORANGE_BANNER, DyeColor::Orange),
        (&*vanilla_items::MAGENTA_BANNER, DyeColor::Magenta),
        (&*vanilla_items::LIGHT_BLUE_BANNER, DyeColor::LightBlue),
        (&*vanilla_items::YELLOW_BANNER, DyeColor::Yellow),
        (&*vanilla_items::LIME_BANNER, DyeColor::Lime),
        (&*vanilla_items::PINK_BANNER, DyeColor::Pink),
        (&*vanilla_items::GRAY_BANNER, DyeColor::Gray),
        (&*vanilla_items::LIGHT_GRAY_BANNER, DyeColor::LightGray),
        (&*vanilla_items::CYAN_BANNER, DyeColor::Cyan),
        (&*vanilla_items::PURPLE_BANNER, DyeColor::Purple),
        (&*vanilla_items::BLUE_BANNER, DyeColor::Blue),
        (&*vanilla_items::BROWN_BANNER, DyeColor::Brown),
        (&*vanilla_items::GREEN_BANNER, DyeColor::Green),
        (&*vanilla_items::RED_BANNER, DyeColor::Red),
        (&*vanilla_items::BLACK_BANNER, DyeColor::Black),
    ];
    banners
        .into_iter()
        .find_map(|(item, color)| (stack.item() == item).then_some(color))
        .unwrap_or(DyeColor::White)
}

#[cfg(test)]
mod tests {
    use vecta_registry::data_components::vanilla_components::DYE;
    use vecta_registry::recipe::{CraftingInput, vanilla_recipe_types};
    use vecta_registry::{DyeColor, REGISTRY, init_vanilla_registry, vanilla_items};

    use super::assemble_recipe;
    use vecta_registry::item_stack::ItemStack;

    #[test]
    fn external_crafting_interpreter_assembles_a_generated_dye_recipe() {
        init_vanilla_registry();
        let input = CraftingInput::new(
            2,
            1,
            vec![
                ItemStack::new(&vanilla_items::RED_DYE),
                ItemStack::new(&vanilla_items::BLUE_DYE),
            ],
        );
        let Some(recipe) = REGISTRY
            .recipes
            .find_match(&vanilla_recipe_types::CRAFTING, &input)
        else {
            panic!("red and blue dye should match the purple dye recipe");
        };

        let result = assemble_recipe(recipe, &input);

        assert!(result.is(&vanilla_items::PURPLE_DYE));
        assert_eq!(result.get(DYE), Some(&DyeColor::Purple));
    }

    fn signed_book(generation: i32) -> ItemStack {
        use text_components::TextComponent;
        use vecta_registry::data_components::components::{Filterable, WrittenBookContent};
        use vecta_registry::data_components::vanilla_components::WRITTEN_BOOK_CONTENT;

        let mut stack = ItemStack::new(&vanilla_items::WRITTEN_BOOK);
        stack.set(
            WRITTEN_BOOK_CONTENT,
            WrittenBookContent::new(
                Filterable::pass_through("Title".to_owned()),
                "Author".to_owned(),
                generation,
                vec![Filterable::pass_through(TextComponent::plain("page"))],
                true,
            )
            .expect("test written book should be valid"),
        );
        stack
    }

    #[test]
    fn book_cloning_copies_the_signed_book_and_raises_its_generation() {
        use vecta_registry::data_components::vanilla_components::WRITTEN_BOOK_CONTENT;

        init_vanilla_registry();
        let input = CraftingInput::new(
            2,
            1,
            vec![
                signed_book(0),
                ItemStack::new(&vanilla_items::WRITABLE_BOOK),
            ],
        );
        let Some(recipe) = REGISTRY
            .recipes
            .find_match(&vanilla_recipe_types::CRAFTING, &input)
        else {
            panic!("a signed book and a writable book should match book cloning");
        };

        let result = assemble_recipe(recipe, &input);

        assert!(result.is(&vanilla_items::WRITTEN_BOOK));
        assert_eq!(result.count(), 1);
        let content = result
            .get(WRITTEN_BOOK_CONTENT)
            .expect("the copy keeps written content");
        assert_eq!(content.generation(), 1);
        assert_eq!(content.title().raw(), "Title");
        assert_eq!(content.author(), "Author");
    }

    #[test]
    fn book_cloning_refuses_generations_outside_the_recipe_range() {
        init_vanilla_registry();
        for generation in [2, 3] {
            let input = CraftingInput::new(
                2,
                1,
                vec![
                    signed_book(generation),
                    ItemStack::new(&vanilla_items::WRITABLE_BOOK),
                ],
            );

            assert!(
                REGISTRY
                    .recipes
                    .find_match(&vanilla_recipe_types::CRAFTING, &input)
                    .is_none(),
                "generation {generation} is outside the default 0..=1 range"
            );
        }
    }
}

/// Vanilla `RecipeManager.getSynchronizedItemProperties`.
///
/// Mirrors `finalizeRecipeLoading`: one pass over the recipe registry in
/// registration order, collecting the ingredient of every recipe that feeds a
/// `RecipePropertySet` key. Vecta loads no brewing recipes yet, so both brewing
/// sets stay empty — which is what Vanilla sends for a datapack that has none.
#[must_use]
pub fn synchronized_item_properties() -> Vec<(Identifier, RecipePropertySet)> {
    let mut smithing_template: Vec<&Ingredient> = Vec::new();
    let mut smithing_base: Vec<&Ingredient> = Vec::new();
    let mut smithing_addition: Vec<&Ingredient> = Vec::new();
    let mut furnace_input: Vec<&Ingredient> = Vec::new();
    let mut blast_furnace_input: Vec<&Ingredient> = Vec::new();
    let mut smoker_input: Vec<&Ingredient> = Vec::new();
    let mut campfire_input: Vec<&Ingredient> = Vec::new();

    for recipe in REGISTRY.recipes.iter() {
        if let Some(smithing) = recipe.downcast_data::<SmithingRecipe>() {
            match smithing {
                SmithingRecipe::Transform(transform) => {
                    if let Some(template) = &transform.template {
                        smithing_template.push(template);
                    }
                    smithing_base.push(&transform.base);
                    if let Some(addition) = &transform.addition {
                        smithing_addition.push(addition);
                    }
                }
                SmithingRecipe::Trim(trim) => {
                    smithing_template.push(&trim.template);
                    smithing_base.push(&trim.base);
                    smithing_addition.push(&trim.addition);
                }
            }
            continue;
        }

        let Some(cooking) = recipe.downcast_data::<CookingRecipe>() else {
            continue;
        };
        let recipe_type = recipe.recipe_type().key.clone();
        if recipe_type == *vanilla_recipe_types::SMELTING.key() {
            furnace_input.push(&cooking.ingredient);
        } else if recipe_type == *vanilla_recipe_types::BLASTING.key() {
            blast_furnace_input.push(&cooking.ingredient);
        } else if recipe_type == *vanilla_recipe_types::SMOKING.key() {
            smoker_input.push(&cooking.ingredient);
        } else if recipe_type == *vanilla_recipe_types::CAMPFIRE_COOKING.key() {
            campfire_input.push(&cooking.ingredient);
        }
    }

    RecipePropertySetKey::ALL
        .into_iter()
        .map(|key| {
            let ingredients: &[&Ingredient] = match key {
                RecipePropertySetKey::SmithingTemplate => &smithing_template,
                RecipePropertySetKey::SmithingBase => &smithing_base,
                RecipePropertySetKey::SmithingAddition => &smithing_addition,
                RecipePropertySetKey::FurnaceInput => &furnace_input,
                RecipePropertySetKey::BlastFurnaceInput => &blast_furnace_input,
                RecipePropertySetKey::SmokerInput => &smoker_input,
                RecipePropertySetKey::CampfireInput => &campfire_input,
                RecipePropertySetKey::BrewingInput | RecipePropertySetKey::BrewingReagent => &[],
            };
            (
                key.identifier(),
                RecipePropertySet::from_ingredients(ingredients.iter().copied()),
            )
        })
        .collect()
}

/// Vanilla `RecipeManager.getSynchronizedStonecutterRecipes`.
///
/// Registration order is preserved so the client's filtered list has the same
/// indices as the server's `StonecutterMenu.setupRecipeList`.
#[must_use]
pub fn synchronized_stonecutter_recipes() -> Vec<SingleInputEntry> {
    REGISTRY
        .recipes
        .iter()
        .filter_map(|recipe| recipe.downcast_data::<StonecuttingRecipe>())
        .map(|recipe| SingleInputEntry {
            input: recipe.ingredient.clone(),
            option_display: SlotDisplay::ItemStack(recipe.result.clone()),
        })
        .collect()
}

#[cfg(test)]
mod sync_tests {
    use vecta_registry::item_stack::ItemStack;
    use vecta_registry::recipe::{SingleItemRecipeInput, SlotDisplay, vanilla_recipe_types};
    use vecta_registry::{REGISTRY, init_vanilla_registry};

    use super::{synchronized_item_properties, synchronized_stonecutter_recipes};

    #[test]
    fn synchronized_item_properties_cover_every_vanilla_key() {
        init_vanilla_registry();

        let sets = synchronized_item_properties();

        assert_eq!(sets.len(), 9, "Vanilla synchronizes nine property sets");
        let smithing_base = sets
            .iter()
            .find(|(key, _)| key.path == "smithing_base")
            .map(|(_, set)| set)
            .expect("smithing_base must be synchronized");
        assert!(
            !smithing_base.items().is_empty(),
            "vanilla ships smithing recipes with a base ingredient"
        );
    }

    /// The client filters the received list by the held input and shows the
    /// results by index, so the synced order has to match the server's own
    /// `StonecutterMenu.setupRecipeList` order.
    #[test]
    fn synchronized_stonecutter_list_matches_server_selection_order() {
        init_vanilla_registry();

        let synced = synchronized_stonecutter_recipes();
        assert!(!synced.is_empty(), "vanilla ships stonecutting recipes");

        let input_item = synced[0]
            .input
            .get_items()
            .first()
            .copied()
            .expect("the first entry must accept at least one item");
        let stack = ItemStack::new(input_item);

        let client: Vec<_> = synced
            .iter()
            .filter(|entry| entry.input.test(&stack))
            .collect();
        let server: Vec<_> = REGISTRY
            .recipes
            .matching(
                &vanilla_recipe_types::STONECUTTING,
                &SingleItemRecipeInput::new(stack),
            )
            .collect();

        assert!(!client.is_empty());
        assert_eq!(client.len(), server.len());
        for (entry, recipe) in client.iter().zip(&server) {
            let SlotDisplay::ItemStack(template) = &entry.option_display else {
                panic!("stonecutter displays must be item stacks");
            };
            assert_eq!(template, &recipe.data().result);
        }
    }
}
