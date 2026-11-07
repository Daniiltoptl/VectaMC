//! Shared furnace, blast-furnace, and smoker block-entity implementation.

use std::array::from_fn;
use std::mem;
use std::sync::{Arc, Weak};

use glam::DVec3;
use rustc_hash::FxHashMap;
use simdnbt::ToNbtTag as _;
use simdnbt::borrow::{BaseNbtCompound as BorrowedNbtCompound, NbtCompound as NbtCompoundView};
use simdnbt::owned::{NbtCompound, NbtList, NbtTag};
use vecta_registry::block_entity_type::BlockEntityTypeRef;
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::blocks::properties::{BlockStateProperties, BoolProperty, Direction};
use vecta_registry::data_components::components::{ResolvableFloat, ResolvableInt};
use vecta_registry::data_components::vanilla_components::COOKING_FUEL;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::loot_table::BlockEntityRef;
use vecta_registry::loot_table::{LootContext, loot_context_params};
use vecta_registry::recipe::{
    CachedRecipeCheck, CookingRecipe, RecipeType, SingleItemRecipeInput, TypedRecipeRef,
    vanilla_recipe_types,
};
use vecta_registry::vanilla_item_tags::ItemTag;
use vecta_registry::{REGISTRY, vanilla_block_entity_types, vanilla_items};
use vecta_utils::locks::IntoShared;
use vecta_utils::nbt::NbtNumeric as _;
use vecta_utils::nbt::NbtValueInput as _;
use vecta_utils::{
    BlockPos, BlockStateId, DowncastType, DowncastTypeKey, Identifier, locks::SyncMutex,
    types::UpdateFlags,
};

use crate::block_entity::{BlockEntity, BlockEntityBase};
use crate::entity::entities::ExperienceOrbEntity;
use crate::inventory::container::Container;
use vecta_utils::nbt::NbtValueInput as _;
use crate::inventory::lock::{ContainerRef, SharedContainer};
use crate::world::World;

/// Vanilla `AbstractFurnaceBlockEntity` container size: input, fuel, result.
pub const FURNACE_SLOTS: usize = 3;
/// Vanilla `AbstractFurnaceBlockEntity.SLOT_INPUT`.
pub const SLOT_INPUT: usize = 0;
/// Vanilla `AbstractFurnaceBlockEntity.SLOT_FUEL`.
pub const SLOT_FUEL: usize = 1;
/// Vanilla `AbstractFurnaceBlockEntity.SLOT_RESULT`.
pub const SLOT_RESULT: usize = 2;

const SLOTS_FOR_UP: &[usize] = &[SLOT_INPUT];
const SLOTS_FOR_DOWN: &[usize] = &[SLOT_RESULT, SLOT_FUEL];
const SLOTS_FOR_SIDES: &[usize] = &[SLOT_FUEL];
const DEFAULT_COOKING_TIME: i32 = 200;
const BURN_COOL_SPEED: i32 = 2;
const LIT: &BoolProperty = &BlockStateProperties::LIT;

/// The operational differences between Vanilla's three furnace block entities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FurnaceKind {
    /// Standard smelting furnace.
    Furnace,
    /// Ore and metal blasting furnace.
    BlastFurnace,
    /// Food smoking furnace.
    Smoker,
}

impl FurnaceKind {
    /// Returns the recipe type processed by this furnace.
    #[must_use]
    pub const fn recipe_type(self) -> &'static RecipeType<CookingRecipe, SingleItemRecipeInput> {
        match self {
            Self::Furnace => &vanilla_recipe_types::SMELTING,
            Self::BlastFurnace => &vanilla_recipe_types::BLASTING,
            Self::Smoker => &vanilla_recipe_types::SMOKING,
        }
    }
}

/// Evaluates a fuel item's `minecraft:cooking_fuel` providers.
///
/// Vanilla builds a `CONTAINER_PROCESS` context from the block state, the block
/// entity, the block centre and the container. Vecta supplies the block state,
/// the centre and the block entity identity; `CONTAINER` has no value type yet
/// (`ContextValueKind::Container` is registry metadata only), so a provider
/// that reads it sees it as missing.
pub(crate) struct FurnaceProviders<'a> {
    world: &'a Arc<World>,
    pos: BlockPos,
    state: BlockStateId,
    block_entity_type: BlockEntityTypeRef,
}

impl<'a> FurnaceProviders<'a> {
    /// Captures the block state and block entity identity the fuel providers
    /// are evaluated against.
    pub(crate) const fn new(
        world: &'a Arc<World>,
        pos: BlockPos,
        state: BlockStateId,
        block_entity_type: BlockEntityTypeRef,
    ) -> Self {
        Self {
            world,
            pos,
            state,
            block_entity_type,
        }
    }

    fn with_context<T>(&self, f: impl FnOnce(&mut LootContext<'_>) -> T) -> T {
        let center = DVec3::new(
            f64::from(self.pos.x()) + 0.5,
            f64::from(self.pos.y()) + 0.5,
            f64::from(self.pos.z()) + 0.5,
        );
        self.world.random().with_level(|rng| {
            let mut ctx = LootContext::new(rng)
                .with_parameter(loot_context_params::BLOCK_STATE, self.state)
                .with_parameter(
                    loot_context_params::BLOCK_ENTITY,
                    BlockEntityRef {
                        block_entity_type: Some(&self.block_entity_type.key),
                        // Vanilla passes the live `Nameable` block entity; the
                        // furnace has no ContainerIdentity in Vecta yet.
                        custom_name: None,
                        inventory: None,
                    },
                )
                .with_parameter(loot_context_params::ORIGIN, (center.x, center.y, center.z))
                .with_data(self.world.as_ref());
            f(&mut ctx)
        })
    }

    /// `ResolvableInt.getFromItem(fuel, COOKING_FUEL, burn_time, ctx, 0)`.
    /// Vanilla `ResolvableInt.get`: constants return without building a
    /// context; a missing reference falls back to the default.
    pub(crate) fn burn_duration(&self, fuel: &ItemStack) -> i32 {
        let Some(component) = fuel.get(COOKING_FUEL) else {
            return 0;
        };
        let key = match &component.burn_time {
            ResolvableInt::Constant(value) => return *value,
            ResolvableInt::Reference(key) => key,
        };
        let Some(provider) = REGISTRY.context_int_providers.by_key(key) else {
            return 0;
        };
        self.with_context(|ctx| provider.get_int(ctx))
    }

    /// `ResolvableFloat.getFromItem(fuel, COOKING_FUEL, speed_multiplier, ctx, 1.0)`.
    pub(crate) fn speed_multiplier(&self, fuel: &ItemStack) -> f32 {
        let Some(component) = fuel.get(COOKING_FUEL) else {
            return 1.0;
        };
        let key = match &component.speed_multiplier {
            ResolvableFloat::Constant(value) => return *value,
            ResolvableFloat::Reference(key) => key,
        };
        let Some(provider) = REGISTRY.context_float_providers.by_key(key) else {
            return 1.0;
        };
        self.with_context(|ctx| provider.get_float(ctx))
    }
}

/// Independently lockable furnace inventory and progress data.
pub struct FurnaceContainer {
    items: [ItemStack; FURNACE_SLOTS],
    lit_time_remaining: i32,
    lit_total_time: i32,
    cooking_timer: i32,
    cooking_total_time: i32,
    speed_multiplier: f32,
    recipes_used: FxHashMap<Identifier, i32>,
    quick_check: CachedRecipeCheck<CookingRecipe, SingleItemRecipeInput>,
}

struct FurnaceTickResult {
    changed: bool,
    lit_changed: bool,
    is_lit: bool,
}

// SAFETY: This Vecta-owned key uniquely identifies furnace inventory/progress storage.
unsafe impl DowncastType for FurnaceContainer {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:container/abstract_furnace");
}

impl FurnaceContainer {
    fn new(kind: FurnaceKind) -> Self {
        Self {
            items: from_fn(|_| ItemStack::empty()),
            lit_time_remaining: 0,
            lit_total_time: 0,
            cooking_timer: 0,
            cooking_total_time: 0,
            speed_multiplier: 1.0,
            recipes_used: FxHashMap::default(),
            quick_check: CachedRecipeCheck::new(kind.recipe_type()),
        }
    }

    #[must_use]
    pub(crate) const fn data(&self) -> [i16; 4] {
        [
            self.lit_time_remaining as i16,
            self.lit_total_time as i16,
            self.cooking_timer as i16,
            self.cooking_total_time as i16,
        ]
    }

    fn recipe_for_input(&mut self) -> Option<TypedRecipeRef<CookingRecipe, SingleItemRecipeInput>> {
        let input = SingleItemRecipeInput::new(self.items[SLOT_INPUT].clone());
        self.quick_check.find_match(&REGISTRY.recipes, &input)
    }

    fn reset_cooking_for_input(&mut self) {
        let recipe_time = self
            .recipe_for_input()
            .map_or(DEFAULT_COOKING_TIME, |recipe| recipe.data().cooking_time);
        self.cooking_total_time = self.total_cook_time(recipe_time);
        self.cooking_timer = 0;
    }

    /// Vanilla `getTotalCookTime`: the fuel's speed multiplier divides the
    /// recipe time, rounded up; a non-positive multiplier keeps the raw time.
    fn total_cook_time(&self, recipe_time: i32) -> i32 {
        if self.speed_multiplier > 0.0 {
            (recipe_time as f32 / self.speed_multiplier).ceil() as i32
        } else {
            recipe_time
        }
    }

    fn can_burn(&self, result: &ItemStack) -> bool {
        if result.is_empty() {
            return false;
        }
        let current = &self.items[SLOT_RESULT];
        if current.is_empty() {
            return true;
        }
        if !ItemStack::is_same_item_same_components(current, result) {
            return false;
        }
        current.count() + result.count() <= self.get_max_stack_size_for_item(result)
    }

    /// Vanilla `consumeFuel`: a crafting remainder replaces the last item, and is
    /// dropped in the world when the fuel stack still holds items.
    fn consume_fuel(&mut self, world: &Arc<World>, pos: BlockPos) {
        let fuel_item = self.items[SLOT_FUEL].item();
        self.items[SLOT_FUEL].shrink(1);
        let remainder = fuel_item.get_crafting_remainder();
        if remainder.is_empty() {
            return;
        }
        if self.items[SLOT_FUEL].is_empty() {
            self.items[SLOT_FUEL] = remainder;
        } else {
            world.drop_item_stack(pos, remainder);
        }
    }

    fn burn(&mut self, recipe: TypedRecipeRef<CookingRecipe, SingleItemRecipeInput>) {
        let result = recipe.data().result.create();
        if self.items[SLOT_RESULT].is_empty() {
            self.items[SLOT_RESULT] = result;
        } else {
            self.items[SLOT_RESULT].grow(result.count());
        }

        if self.items[SLOT_INPUT].is(&vanilla_items::WET_SPONGE)
            && self.items[SLOT_FUEL].is(&vanilla_items::BUCKET)
        {
            self.items[SLOT_FUEL] = ItemStack::new(&vanilla_items::WATER_BUCKET);
        }
        self.items[SLOT_INPUT].shrink(1);
        *self.recipes_used.entry(recipe.key().clone()).or_default() += 1;
    }

    fn tick(&mut self, providers: &FurnaceProviders<'_>) -> FurnaceTickResult {
        let was_lit = self.lit_time_remaining > 0;
        if was_lit {
            self.lit_time_remaining -= 1;
        }
        let mut is_lit = self.lit_time_remaining > 0;
        let mut changed = false;
        let has_ingredient = !self.items[SLOT_INPUT].is_empty();
        let has_fuel = !self.items[SLOT_FUEL].is_empty();

        if is_lit || (has_fuel && has_ingredient) {
            if let Some(recipe) = self.recipe_for_input() {
                let result = recipe.data().result.create();
                if self.can_burn(&result) {
                    if !is_lit {
                        // Vanilla stores both timers and the multiplier before it
                        // checks whether the fuel actually burns.
                        let duration = providers.burn_duration(&self.items[SLOT_FUEL]);
                        let speed_multiplier = providers.speed_multiplier(&self.items[SLOT_FUEL]);
                        self.lit_time_remaining = duration;
                        self.lit_total_time = duration;
                        self.speed_multiplier = speed_multiplier;
                        if self.cooking_total_time > 0
                            && self.cooking_timer < self.cooking_total_time
                        {
                            // Keep the completion ratio when the fuel changes the
                            // cooking speed mid-recipe.
                            let completion_ratio =
                                self.cooking_timer as f32 / self.cooking_total_time as f32;
                            self.cooking_total_time =
                                self.total_cook_time(recipe.data().cooking_time);
                            self.cooking_timer =
                                (completion_ratio * self.cooking_total_time as f32).ceil() as i32;
                        }
                        if duration > 0 {
                            self.consume_fuel(providers.world, providers.pos);
                            is_lit = true;
                            changed = true;
                        }
                    }

                    if is_lit {
                        self.cooking_timer += 1;
                        if self.cooking_timer >= self.cooking_total_time {
                            self.cooking_timer = 0;
                            self.cooking_total_time =
                                self.total_cook_time(recipe.data().cooking_time);
                            self.burn(recipe);
                            changed = true;
                        }
                    } else {
                        self.cooking_timer = 0;
                    }
                } else {
                    self.cooking_timer = 0;
                }
            } else {
                self.cooking_timer = 0;
            }
        } else if self.cooking_timer > 0 {
            self.cooking_timer =
                (self.cooking_timer - BURN_COOL_SPEED).clamp(0, self.cooking_total_time);
        }

        FurnaceTickResult {
            changed,
            lit_changed: was_lit != is_lit,
            is_lit,
        }
    }

    pub(crate) fn take_recipes_used(&mut self) -> FxHashMap<Identifier, i32> {
        mem::take(&mut self.recipes_used)
    }
}

impl Container for FurnaceContainer {
    fn items(&self) -> &[ItemStack] {
        &self.items
    }

    fn items_mut(&mut self) -> &mut [ItemStack] {
        &mut self.items
    }

    fn set_item(&mut self, slot: usize, mut stack: ItemStack) {
        if slot >= FURNACE_SLOTS {
            return;
        }
        let same_input = slot == SLOT_INPUT
            && !stack.is_empty()
            && ItemStack::is_same_item_same_components(&self.items[slot], &stack);
        let max_size = self.get_max_stack_size_for_item(&stack);
        if stack.count() > max_size {
            stack.set_count(max_size);
        }
        self.items[slot] = stack;
        if slot == SLOT_INPUT && !same_input {
            self.reset_cooking_for_input();
        }
    }

    fn get_max_stack_size(&self) -> i32 {
        64
    }

    fn set_changed(&mut self) {}

    fn can_place_item(&self, slot: usize, stack: &ItemStack) -> bool {
        match slot {
            SLOT_RESULT => false,
            SLOT_FUEL => {
                stack.get(COOKING_FUEL).is_some()
                    || (stack.is(&vanilla_items::BUCKET)
                        && !self.items[SLOT_FUEL].is(&vanilla_items::BUCKET))
            }
            _ => true,
        }
    }

    fn slots_for_face(&self, direction: Direction) -> Option<&'static [usize]> {
        Some(match direction {
            Direction::Down => SLOTS_FOR_DOWN,
            Direction::Up => SLOTS_FOR_UP,
            _ => SLOTS_FOR_SIDES,
        })
    }

    fn can_take_item_through_face(
        &self,
        slot: usize,
        stack: &ItemStack,
        direction: Direction,
    ) -> bool {
        // Vanilla only restricts the fuel slot from below, and only to the
        // items in `#furnace_fuel_bottom_takeable` (the two buckets).
        direction != Direction::Down
            || slot != SLOT_FUEL
            || stack.item().has_tag(&ItemTag::FURNACE_FUEL_BOTTOM_TAKEABLE)
    }
}

/// Shared implementation owned by one of the three concrete furnace entities.
pub struct AbstractFurnaceBlockEntity {
    base: Arc<BlockEntityBase>,
    container: Arc<SyncMutex<FurnaceContainer>>,
    container_ref: ContainerRef,
}

impl AbstractFurnaceBlockEntity {
    fn new(
        block_entity_type: BlockEntityTypeRef,
        kind: FurnaceKind,
        level: Weak<World>,
        pos: BlockPos,
        state: BlockStateId,
    ) -> Self {
        let base = Arc::new(BlockEntityBase::new(block_entity_type, level, pos, state));
        let container = FurnaceContainer::new(kind).into_shared();
        let shared: SharedContainer = Arc::<SyncMutex<FurnaceContainer>>::clone(&container);
        Self {
            container_ref: ContainerRef::owned_by_block_entity(shared, Arc::clone(&base)),
            base,
            container,
        }
    }

    #[must_use]
    pub fn container_ref(&self) -> ContainerRef {
        self.container_ref.clone()
    }

    fn server_tick(&self, world: &Arc<World>) {
        let providers = FurnaceProviders::new(
            world,
            self.base.pos(),
            self.base.block_state(),
            self.base.block_entity_type(),
        );
        let result = self.container.lock().tick(&providers);

        if result.lit_changed {
            let state = self.base.block_state().set_value(LIT, result.is_lit);
            world.set_block(self.base.pos(), state, UpdateFlags::UPDATE_ALL);
        }
        if result.changed || result.lit_changed {
            self.base.set_changed();
        }
    }

    fn load_additional(&self, nbt: &BorrowedNbtCompound<'_>) {
        let nbt: NbtCompoundView<'_, '_> = nbt.into();
        let mut furnace = self.container.lock();
        furnace.items.fill(ItemStack::empty());
        if let Some(items) = nbt.list("Items").and_then(|list| list.compounds()) {
            for compound in items {
                let Some(slot) = compound.byte("Slot").map(|slot| slot as usize) else {
                    continue;
                };
                if slot < FURNACE_SLOTS
                    && let Some(stack) = ItemStack::from_borrowed_compound(&compound)
                {
                    furnace.items[slot] = stack;
                }
            }
        }
        // 26.3 stores the timers as ints and keeps the fuel's speed multiplier.
        furnace.cooking_timer = nbt.int_value("cooking_time_spent").unwrap_or(0);
        furnace.cooking_total_time = nbt.int_value("cooking_total_time").unwrap_or(0);
        furnace.lit_time_remaining = nbt.int_value("lit_time_remaining").unwrap_or(0);
        furnace.lit_total_time = nbt.int_value("lit_total_time").unwrap_or(0);
        furnace.speed_multiplier = nbt.float_value("speed_multiplier").unwrap_or(1.0);
        furnace.recipes_used.clear();
        if let Some(recipes) = nbt.compound("RecipesUsed") {
            for (key, value) in recipes.iter() {
                let Some(count) = value.codec_i32() else {
                    continue;
                };
                if let Ok(identifier) = key.to_string().parse() {
                    furnace.recipes_used.insert(identifier, count);
                }
            }
        }
    }

    fn save_additional(&self, nbt: &mut NbtCompound) {
        let furnace = self.container.lock();
        nbt.insert("cooking_time_spent", furnace.cooking_timer);
        nbt.insert("cooking_total_time", furnace.cooking_total_time);
        nbt.insert("speed_multiplier", furnace.speed_multiplier);
        nbt.insert("lit_time_remaining", furnace.lit_time_remaining);
        nbt.insert("lit_total_time", furnace.lit_total_time);

        let mut items = Vec::new();
        for (slot, stack) in furnace.items.iter().enumerate() {
            if !stack.is_empty()
                && let NbtTag::Compound(mut item) = stack.clone().to_nbt_tag()
            {
                item.insert("Slot", slot as i8);
                items.push(item);
            }
        }
        nbt.insert("Items", NbtList::Compound(items));

        let mut recipes = NbtCompound::new();
        for (key, count) in &furnace.recipes_used {
            recipes.insert(key.to_string(), *count);
        }
        nbt.insert("RecipesUsed", recipes);
    }

    fn pre_remove_side_effects(&self, world: &Arc<World>, pos: BlockPos) {
        let (items, recipes) = {
            let mut furnace = self.container.lock();
            let items = mem::replace(&mut furnace.items, from_fn(|_| ItemStack::empty()));
            (items, furnace.take_recipes_used())
        };
        for item in items {
            world.drop_item_stack(pos, item);
        }
        pop_furnace_experience(
            world,
            DVec3::new(
                f64::from(pos.x()) + 0.5,
                f64::from(pos.y()) + 0.5,
                f64::from(pos.z()) + 0.5,
            ),
            recipes,
        );
    }
}

pub(crate) fn pop_furnace_experience(
    world: &Arc<World>,
    position: DVec3,
    recipes: FxHashMap<Identifier, i32>,
) {
    for (key, amount) in recipes {
        let Some(recipe) = REGISTRY.recipes.by_key(&key) else {
            continue;
        };
        let Some(cooking) = recipe.downcast_data::<CookingRecipe>() else {
            continue;
        };
        let exact = amount as f32 * cooking.experience;
        let mut reward = exact.floor() as i32;
        if exact.fract() != 0.0 && rand::random::<f32>() < exact.fract() {
            reward += 1;
        }
        ExperienceOrbEntity::award(world, position, reward);
    }
}

macro_rules! furnace_block_entity {
    ($name:ident, $key:literal, $type:ident, $kind:ident) => {
        #[doc = concat!(
                                            "`",
                                            stringify!($name),
                                            "` block entity backed by the shared smelting logic."
                                        )]
        pub struct $name {
            common: AbstractFurnaceBlockEntity,
        }

        // SAFETY: This Vecta-owned key uniquely identifies this concrete block entity.
        unsafe impl DowncastType for $name {
            const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new($key);
        }

        impl $name {
            /// Creates the block entity at a live world position.
            #[must_use]
            pub fn new(level: Weak<World>, pos: BlockPos, state: BlockStateId) -> Self {
                Self {
                    common: AbstractFurnaceBlockEntity::new(
                        &vanilla_block_entity_types::$type,
                        FurnaceKind::$kind,
                        level,
                        pos,
                        state,
                    ),
                }
            }
        }

        impl BlockEntity for $name {
            fn base(&self) -> &BlockEntityBase {
                &self.common.base
            }

            fn pre_remove_side_effects(&self, pos: BlockPos, _state: BlockStateId) {
                if let Some(world) = self.get_level() {
                    self.common.pre_remove_side_effects(&world, pos);
                }
            }

            fn load_additional(&self, nbt: &BorrowedNbtCompound<'_>) {
                self.common.load_additional(nbt);
            }

            fn save_additional(&self, nbt: &mut NbtCompound) {
                self.common.save_additional(nbt);
            }

            fn tick(&self, world: &Arc<World>) {
                self.common.server_tick(world);
            }

            fn container_ref(&self) -> Option<ContainerRef> {
                Some(self.common.container_ref())
            }
        }
    };
}

furnace_block_entity!(
    FurnaceBlockEntity,
    "vecta:block_entity/furnace",
    FURNACE,
    Furnace
);
furnace_block_entity!(
    BlastFurnaceBlockEntity,
    "vecta:block_entity/blast_furnace",
    BLAST_FURNACE,
    BlastFurnace
);
furnace_block_entity!(
    SmokerBlockEntity,
    "vecta:block_entity/smoker",
    SMOKER,
    Smoker
);

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use simdnbt::borrow::read_compound as read_borrowed_compound;
    use vecta_registry::{init_vanilla_registry, vanilla_blocks, vanilla_items};

    use super::*;
    use crate::test_support::fresh_test_world;

    /// The block state the fuel providers are evaluated against.
    fn kind_state(kind: FurnaceKind) -> BlockStateId {
        match kind {
            FurnaceKind::Furnace => vanilla_blocks::FURNACE.default_state(),
            FurnaceKind::BlastFurnace => vanilla_blocks::BLAST_FURNACE.default_state(),
            FurnaceKind::Smoker => vanilla_blocks::SMOKER.default_state(),
        }
    }

    /// The block entity identity the fuel providers are evaluated against.
    fn kind_block_entity_type(kind: FurnaceKind) -> BlockEntityTypeRef {
        match kind {
            FurnaceKind::Furnace => &vanilla_block_entity_types::FURNACE,
            FurnaceKind::BlastFurnace => &vanilla_block_entity_types::BLAST_FURNACE,
            FurnaceKind::Smoker => &vanilla_block_entity_types::SMOKER,
        }
    }

    /// Vanilla `BaseContainerBlockEntity.getLootContext` passes
    /// `LootContextParams.BLOCK_ENTITY = this` for the `CONTAINER_PROCESS`
    /// fuel providers.
    #[test]
    fn the_fuel_context_carries_the_block_entity_identity() {
        init_vanilla_registry();
        let fixture = fresh_test_world("furnace_context_block_entity");
        let providers = FurnaceProviders::new(
            &fixture.world,
            BlockPos::ZERO,
            kind_state(FurnaceKind::BlastFurnace),
            kind_block_entity_type(FurnaceKind::BlastFurnace),
        );

        let block_entity_type = providers.with_context(|ctx| {
            ctx.params
                .get(loot_context_params::BLOCK_ENTITY)
                .and_then(|block_entity| block_entity.block_entity_type.map(ToString::to_string))
        });
        assert_eq!(
            block_entity_type.as_deref(),
            Some("minecraft:blast_furnace"),
            "the fuel context must expose the furnace block entity identity"
        );
    }

    fn cook_iron(kind: FurnaceKind, expected_ticks: usize, expected_fuel_time: i32) {
        init_vanilla_registry();
        let fixture = fresh_test_world("furnace_cook_iron");
        let world = &fixture.world;
        let providers = FurnaceProviders::new(
            world,
            BlockPos::ZERO,
            kind_state(kind),
            kind_block_entity_type(kind),
        );
        let mut furnace = FurnaceContainer::new(kind);
        furnace.set_item(SLOT_INPUT, ItemStack::new(&vanilla_items::IRON_ORE));
        furnace.set_item(SLOT_FUEL, ItemStack::new(&vanilla_items::COAL));

        for _ in 0..expected_ticks {
            furnace.tick(&providers);
        }

        assert!(furnace.items[SLOT_INPUT].is_empty());
        assert!(furnace.items[SLOT_FUEL].is_empty());
        assert!(furnace.items[SLOT_RESULT].is(&vanilla_items::IRON_INGOT));
        assert_eq!(furnace.items[SLOT_RESULT].count(), 1);
        assert_eq!(furnace.lit_total_time, expected_fuel_time);
        assert_eq!(furnace.recipes_used.values().sum::<i32>(), 1);
    }

    #[test]
    fn total_cook_time_uses_the_fuel_speed_multiplier() {
        let mut furnace = FurnaceContainer::new(FurnaceKind::Furnace);
        assert_eq!(furnace.total_cook_time(200), 200);
        furnace.speed_multiplier = 2.0;
        assert_eq!(furnace.total_cook_time(200), 100);
        furnace.speed_multiplier = 3.0;
        assert_eq!(furnace.total_cook_time(200), 67, "ceil(200 / 3)");
        furnace.speed_multiplier = 0.0;
        assert_eq!(
            furnace.total_cook_time(200),
            200,
            "a non-positive multiplier keeps the raw recipe time"
        );
    }

    #[test]
    fn relighting_keeps_the_progress_in_float_precision() {
        // Java multiplies `float * int`: 0.025F * 200 rounds to exactly 5, while
        // the same product in double is just above it and would ceil to 6.
        init_vanilla_registry();
        let fixture = fresh_test_world("furnace_relight_ratio");
        let providers = FurnaceProviders::new(
            &fixture.world,
            BlockPos::ZERO,
            kind_state(FurnaceKind::Furnace),
            kind_block_entity_type(FurnaceKind::Furnace),
        );
        let mut furnace = FurnaceContainer::new(FurnaceKind::Furnace);
        furnace.set_item(SLOT_INPUT, ItemStack::new(&vanilla_items::IRON_ORE));
        furnace.set_item(SLOT_FUEL, ItemStack::new(&vanilla_items::COAL));
        furnace.cooking_timer = 5;
        furnace.cooking_total_time = 200;

        furnace.tick(&providers);

        assert_eq!(furnace.cooking_timer, 6, "the kept 5 plus this tick");
    }

    #[test]
    fn the_burn_timers_are_stored_as_ints() {
        init_vanilla_registry();
        let furnace = AbstractFurnaceBlockEntity::new(
            &vanilla_block_entity_types::FURNACE,
            FurnaceKind::Furnace,
            Weak::new(),
            BlockPos::ZERO,
            kind_state(FurnaceKind::Furnace),
        );
        let mut nbt = NbtCompound::new();
        nbt.insert("lit_time_remaining", 40_000);
        nbt.insert("lit_total_time", 40_001);
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed = read_borrowed_compound(&mut Cursor::new(bytes.as_slice()))
            .expect("furnace NBT decodes");

        furnace.load_additional(&borrowed);
        let mut saved = NbtCompound::new();
        furnace.save_additional(&mut saved);

        assert_eq!(saved.int("lit_time_remaining"), Some(40_000));
        assert_eq!(saved.int("lit_total_time"), Some(40_001));
    }

    #[test]
    fn a_non_fuel_item_never_lights_the_furnace() {
        init_vanilla_registry();
        let fixture = fresh_test_world("furnace_non_fuel");
        let providers = FurnaceProviders::new(
            &fixture.world,
            BlockPos::ZERO,
            kind_state(FurnaceKind::Furnace),
            kind_block_entity_type(FurnaceKind::Furnace),
        );
        let mut furnace = FurnaceContainer::new(FurnaceKind::Furnace);
        furnace.set_item(SLOT_INPUT, ItemStack::new(&vanilla_items::IRON_ORE));
        furnace.set_item(SLOT_FUEL, ItemStack::new(&vanilla_items::DIRT));

        let result = furnace.tick(&providers);
        assert!(
            !result.is_lit,
            "an item without `cooking_fuel` never lights"
        );
        assert_eq!(
            furnace.items[SLOT_FUEL].count(),
            1,
            "a non-fuel item is not consumed"
        );
    }

    #[test]
    fn furnace_consumes_one_fuel_and_finishes_at_recipe_cooking_time() {
        cook_iron(FurnaceKind::Furnace, 200, 1600);
    }

    #[test]
    fn blast_furnace_uses_blasting_time_and_half_fuel_duration() {
        cook_iron(FurnaceKind::BlastFurnace, 100, 800);
    }

    #[test]
    fn furnace_sided_inventory_matches_vanilla_faces_and_bucket_rule() {
        init_vanilla_registry();
        let mut furnace = FurnaceContainer::new(FurnaceKind::Furnace);

        assert_eq!(
            furnace.slots_for_face(Direction::Up),
            Some(&[SLOT_INPUT][..])
        );
        assert_eq!(
            furnace.slots_for_face(Direction::Down),
            Some(&[SLOT_RESULT, SLOT_FUEL][..])
        );
        assert_eq!(
            furnace.slots_for_face(Direction::North),
            Some(&[SLOT_FUEL][..])
        );
        assert!(!furnace.can_take_item_through_face(
            SLOT_FUEL,
            &ItemStack::new(&vanilla_items::COAL),
            Direction::Down,
        ));
        assert!(furnace.can_take_item_through_face(
            SLOT_FUEL,
            &ItemStack::new(&vanilla_items::WATER_BUCKET),
            Direction::Down,
        ));
        assert!(furnace.can_take_item_through_face(
            SLOT_FUEL,
            &ItemStack::new(&vanilla_items::BUCKET),
            Direction::Down,
        ));
        assert!(
            furnace.can_take_item_through_face(
                SLOT_RESULT,
                &ItemStack::new(&vanilla_items::IRON_INGOT),
                Direction::Down,
            ),
            "the result slot is only reachable from below"
        );
        assert!(
            furnace.can_take_item_through_face(
                SLOT_FUEL,
                &ItemStack::new(&vanilla_items::COAL),
                Direction::North,
            ),
            "the bottom-only rule does not restrict a side face"
        );

        // `canPlaceItem`: the result slot never accepts, the fuel slot takes
        // fuel and one empty bucket, and the input slot takes anything.
        assert!(!furnace.can_place_item(SLOT_RESULT, &ItemStack::new(&vanilla_items::IRON_INGOT)));
        assert!(furnace.can_place_item(SLOT_FUEL, &ItemStack::new(&vanilla_items::COAL)));
        assert!(!furnace.can_place_item(SLOT_FUEL, &ItemStack::new(&vanilla_items::IRON_INGOT)));
        assert!(
            furnace.can_place_item(SLOT_FUEL, &ItemStack::new(&vanilla_items::BUCKET)),
            "an empty bucket is a valid fuel-slot item while that slot is empty"
        );
        furnace.set_item(SLOT_FUEL, ItemStack::new(&vanilla_items::BUCKET));
        assert!(
            !furnace.can_place_item(SLOT_FUEL, &ItemStack::new(&vanilla_items::BUCKET)),
            "vanilla refuses a second bucket in the fuel slot"
        );
        assert!(furnace.can_place_item(SLOT_INPUT, &ItemStack::new(&vanilla_items::IRON_ORE)));

        // Vanilla `canPlaceItemThroughFace` is exactly `canPlaceItem`.
        for slot in [SLOT_INPUT, SLOT_FUEL, SLOT_RESULT] {
            for stack in [
                ItemStack::new(&vanilla_items::IRON_ORE),
                ItemStack::new(&vanilla_items::COAL),
                ItemStack::new(&vanilla_items::BUCKET),
            ] {
                assert_eq!(
                    furnace.can_place_item_through_face(slot, &stack, Direction::Up),
                    furnace.can_place_item(slot, &stack),
                    "slot {slot} must not add a face rule on insertion"
                );
            }
        }
    }
}
