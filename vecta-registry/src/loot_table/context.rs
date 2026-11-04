use super::{DyeColor, Identifier, ItemStack, REGISTRY, RegistryExt};
use crate::equipment::EquipmentSlot;
use vecta_utils::nbt::NbtPath;

mod data;
mod map;
mod parameter_sets;
mod parameters;
pub use crate::vanilla_context_key_sets::params as loot_context_params;
pub use data::*;
pub use map::*;
pub use parameter_sets::*;
pub use parameters::*;
use vecta_utils::random::{Random as _, RandomSource};

/// Entity target for loot context lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootContextEntity {
    /// The entity being looted (killed mob, block entity owner).
    This,
    /// The entity that killed the target.
    Killer,
    /// The direct attacker (e.g., arrow, not the player who shot it).
    DirectKiller,
    /// The player who dealt the final damage.
    KillerPlayer,
    /// The entity interacting with a block/entity.
    Interacting,
    /// The explicit command/interaction target.
    Target,
}

/// The type of loot table, determining when/how it's used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootType {
    Block,
    Entity,
    Chest,
    Fishing,
    Gift,
    Archaeology,
    Vault,
    Shearing,
    Equipment,
    Selector,
    EntityInteract,
    BlockInteract,
    Barter,
}

/// A number provider that can be constant or random.
#[derive(Debug, Clone)]
pub enum NumberProvider {
    Constant(f32),
    Uniform {
        min: f32,
        max: f32,
    },
    Binomial {
        n: i32,
        p: f32,
    },
    /// Get value from entity scoreboard score.
    Score {
        target: ScoreboardTarget,
        score: &'static str,
    },
    /// Get value from command storage.
    Storage {
        storage: Identifier,
        path: NbtPath,
    },
    /// Get enchantment level from context tool.
    EnchantmentLevel {
        enchantment: Identifier,
    },
}

/// Target for scoreboard number provider.
#[derive(Debug, Clone, Copy)]
pub enum ScoreboardTarget {
    /// The entity being looted.
    This,
    /// The entity that killed the target.
    Killer,
    /// The direct killer (e.g., arrow vs player).
    DirectKiller,
    /// The player who dealt the last damage.
    KillerPlayer,
    Target,
    Interacting,
    /// A fixed entity name.
    Fixed(&'static str),
}

impl NumberProvider {
    /// Evaluates against the same parameters and live level services as the table.
    pub fn get_float(&self, ctx: &mut LootContext<'_>) -> f32 {
        match self {
            Self::Score { target, score } => target.get_score(ctx, score).unwrap_or(0) as f32,
            Self::Storage { storage, path } => ctx
                .data
                .and_then(|data| data.storage_float(storage, path))
                .unwrap_or(0.0),
            Self::EnchantmentLevel { enchantment } => {
                ctx.get_enchantment_level_by_id(enchantment) as f32
            }
            other => other.get_simple(ctx.rng),
        }
    }

    /// Get a value without context (for backwards compatibility).
    pub fn get_simple(&self, rng: &mut RandomSource) -> f32 {
        match self {
            Self::Constant(v) => *v,
            Self::Uniform { min, max } => next_float(rng, *min, *max),
            Self::Binomial { n, p } => {
                let mut count = 0;
                for _ in 0..*n {
                    if rng.next_f32() < *p {
                        count += 1;
                    }
                }
                count as f32
            }
            // Context-dependent providers return 0 when no context available
            Self::Score { .. } | Self::Storage { .. } | Self::EnchantmentLevel { .. } => 0.0,
        }
    }

    /// Get the value as an integer.
    pub fn get_int(&self, rng: &mut RandomSource) -> i32 {
        match self {
            Self::Uniform { min, max } => uniform_int(rng, math_round(*min), math_round(*max)),
            other => math_round(other.get_simple(rng)),
        }
    }

    /// Keeps integer score/storage values out of a lossy intermediate float.
    pub fn get_int_with_context(&self, ctx: &mut LootContext<'_>) -> i32 {
        match self {
            Self::Uniform { min, max } => uniform_int(ctx.rng, math_round(*min), math_round(*max)),
            Self::Score { target, score } => target.get_score(ctx, score).unwrap_or(0),
            Self::Storage { storage, path } => ctx
                .data
                .and_then(|data| data.storage_int(storage, path))
                .unwrap_or(0),
            other => math_round(other.get_float(ctx)),
        }
    }
}

impl ScoreboardTarget {
    pub(crate) fn get_score(self, ctx: &LootContext<'_>, objective: &str) -> Option<i32> {
        let holder = match self {
            Self::Fixed(name) => LootScoreHolder::Name(name),
            Self::This => ctx.get_entity(LootContextEntity::This)?.score_holder?,
            Self::Killer => ctx.get_entity(LootContextEntity::Killer)?.score_holder?,
            Self::DirectKiller => {
                ctx.get_entity(LootContextEntity::DirectKiller)?
                    .score_holder?
            }
            Self::KillerPlayer => {
                ctx.get_entity(LootContextEntity::KillerPlayer)?
                    .score_holder?
            }
            Self::Target => ctx.get_entity(LootContextEntity::Target)?.score_holder?,
            Self::Interacting => {
                ctx.get_entity(LootContextEntity::Interacting)?
                    .score_holder?
            }
        };
        ctx.data?.score(holder, objective)
    }
}

/// `java.lang.Math.round` semantics for a float.
pub(crate) fn math_round(value: f32) -> i32 {
    (f64::from(value) + 0.5).floor() as i32
}

/// Vanilla `Mth.nextFloat`: `min >= max` returns `min` without consuming a
/// number, otherwise one `nextFloat` is scaled into `[min, max)`.
pub fn next_float(rng: &mut RandomSource, min: f32, max: f32) -> f32 {
    if min >= max {
        min
    } else {
        rng.next_f32() * (max - min) + min
    }
}

/// Vanilla `Mth.nextInt`: `min >= max` returns `min` without consuming a
/// number, otherwise one bounded draw is made.
pub fn next_int(rng: &mut RandomSource, min: i32, max: i32) -> i32 {
    if min >= max {
        min
    } else {
        rng.next_i32_between(min, max)
    }
}

/// Vanilla `Mth.nextInt(random, min, max)` is inclusive and clamps to `min`
/// when `min >= max`.
fn uniform_int(rng: &mut RandomSource, min: i32, max: i32) -> i32 {
    next_int(rng, min, max)
}

/// A range for number comparisons (used in `ValueCheck`, `TimeCheck`, `EntityScores`).
#[derive(Debug, Clone)]
pub struct NumberProviderRange {
    pub min: Option<NumberProvider>,
    pub max: Option<NumberProvider>,
}

impl NumberProviderRange {
    /// Integer predicates retain exact scoreboard values and boxed-NBT casts.
    pub fn test_int(&self, value: i32, ctx: &mut LootContext<'_>) -> bool {
        if let Some(min) = &self.min
            && value < min.get_int_with_context(ctx)
        {
            return false;
        }
        if let Some(max) = &self.max
            && value > max.get_int_with_context(ctx)
        {
            return false;
        }
        true
    }

    /// Check if a value is within this range.
    pub fn test(&self, value: f32, ctx: &mut LootContext<'_>) -> bool {
        if let Some(min) = &self.min
            && value < min.get_float(ctx)
        {
            return false;
        }
        if let Some(max) = &self.max
            && value > max.get_float(ctx)
        {
            return false;
        }
        true
    }

    /// Create an exact match range.
    #[must_use]
    pub const fn exact(value: f32) -> Self {
        Self {
            min: Some(NumberProvider::Constant(value)),
            max: Some(NumberProvider::Constant(value)),
        }
    }

    /// Create an at-least range.
    #[must_use]
    pub const fn at_least(min: f32) -> Self {
        Self {
            min: Some(NumberProvider::Constant(min)),
            max: None,
        }
    }

    /// Create an at-most range.
    #[must_use]
    pub const fn at_most(max: f32) -> Self {
        Self {
            min: None,
            max: Some(NumberProvider::Constant(max)),
        }
    }

    /// Create a between range.
    #[must_use]
    pub const fn between(min: f32, max: f32) -> Self {
        Self {
            min: Some(NumberProvider::Constant(min)),
            max: Some(NumberProvider::Constant(max)),
        }
    }
}

/// Loot parameters share one typed map; RNG, luck and level state are separate.
pub struct LootContext<'a> {
    pub rng: &'a mut RandomSource,
    pub luck: f32,
    pub params: ContextMap<'a>,
    pub data: Option<&'a dyn LootDataReader>,
    pub game_time: Option<i64>,
    pub weather: Option<WeatherState>,
}

/// Weather state for `WeatherCheck` condition.
#[derive(Debug, Clone, Copy, Default)]
pub struct WeatherState {
    pub raining: bool,
    pub thundering: bool,
}

/// A reference to an entity for loot context.
/// This is intentionally opaque - the actual entity type depends on game implementation.
#[derive(Debug, Clone, Copy)]
pub struct EntityRef<'a> {
    /// Type identifier for the entity.
    pub entity_type: Option<&'a Identifier>,
    /// Score identity is separate from the entity's type and display name.
    pub score_holder: Option<LootScoreHolder<'a>>,
    /// Entity flags for predicate checking.
    pub flags: EntityRefFlags,
    /// Equipment slots for equipment predicates. `None` for non-living entities.
    pub equipment: Option<EntityEquipmentRef<'a>>,
    /// Entity name (for `copy_name` function).
    pub custom_name: Option<&'a str>,
    /// Vanilla `minecraft:components.sheep/color` entity data component.
    pub sheep_color: Option<DyeColor>,
    /// Vanilla `minecraft:type_specific/sheep.sheared`. `None` when the entity is
    /// not a sheep, matching vanilla `SheepPredicate.matches`' non-sheep rejection.
    pub sheep_sheared: Option<bool>,
    /// Vanilla `minecraft:components.chicken/variant` entity data component key.
    pub chicken_variant: Option<&'a Identifier>,
}

/// Entity flags for predicate checking.
#[derive(Debug, Clone, Copy, Default)]
pub struct EntityRefFlags {
    pub is_on_fire: bool,
    pub is_sneaking: bool,
    pub is_sprinting: bool,
    pub is_swimming: bool,
    pub is_baby: bool,
}

/// One item stack per equipment slot, indexed by [`EquipmentSlot::index`].
pub type EntityEquipmentSlots = [ItemStack; EquipmentSlot::ALL.len()];

/// Every equipment slot of a living entity.
#[derive(Debug, Clone, Copy)]
pub struct EntityEquipmentRef<'a>(&'a EntityEquipmentSlots);

impl<'a> EntityEquipmentRef<'a> {
    #[must_use]
    pub const fn new(slots: &'a EntityEquipmentSlots) -> Self {
        Self(slots)
    }

    /// Returns the item in the given equipment slot.
    #[must_use]
    pub const fn get(self, slot: EquipmentSlot) -> &'a ItemStack {
        &self.0[slot.index()]
    }

    /// Returns the highest enchantment level from matching equipment slots.
    #[must_use]
    pub fn get_enchantment_level(self, enchantment: &Identifier) -> i32 {
        let Some(definition) = REGISTRY.enchantments.by_key(enchantment) else {
            return 0;
        };
        EquipmentSlot::ALL
            .into_iter()
            .filter(|slot| definition.matching_slot(*slot))
            .filter_map(|slot| {
                let stack = self.get(slot);
                (!stack.is_empty()).then(|| stack.get_enchantment_level(enchantment))
            })
            .max()
            .unwrap_or(0)
    }
}

/// Damage source information for loot context.
#[derive(Debug, Clone, Copy)]
pub struct DamageSourceInfo<'a> {
    /// The damage type identifier.
    pub damage_type: Option<&'a Identifier>,
    /// Tags associated with this damage source.
    pub tags: &'a [Identifier],
    /// Whether this is direct damage (not from a projectile).
    pub is_direct: bool,
}

/// A reference to a block entity for loot context.
#[derive(Debug, Clone, Copy)]
pub struct BlockEntityRef<'a> {
    /// The block entity type identifier.
    pub block_entity_type: Option<&'a Identifier>,
    /// Custom name of the block entity (for `copy_name`).
    pub custom_name: Option<&'a str>,
    /// Inventory contents (for dynamic/slots entries).
    pub inventory: Option<&'a [ItemStack]>,
}

impl<'a> LootContext<'a> {
    pub fn new(rng: &'a mut RandomSource) -> Self {
        Self {
            rng,
            luck: 0.0,
            params: ContextMap::default(),
            data: None,
            game_time: None,
            weather: None,
        }
    }

    #[must_use]
    pub fn with_data(mut self, data: &'a dyn LootDataReader) -> Self {
        self.data = Some(data);
        self
    }

    #[must_use]
    pub fn with_parameter<T: ContextValueType>(
        mut self,
        key: ContextKey<T>,
        value: T::Value<'a>,
    ) -> Self {
        self.params.set(key, Some(value));
        self
    }

    #[must_use]
    pub fn with_optional_parameter<T: ContextValueType>(
        mut self,
        key: ContextKey<T>,
        value: Option<T::Value<'a>>,
    ) -> Self {
        self.params.set(key, value);
        self
    }

    #[must_use]
    pub const fn with_luck(mut self, luck: f32) -> Self {
        self.luck = luck;
        self
    }

    #[must_use]
    pub const fn with_game_time(mut self, time: i64) -> Self {
        self.game_time = Some(time);
        self
    }

    #[must_use]
    pub const fn with_weather(mut self, weather: WeatherState) -> Self {
        self.weather = Some(weather);
        self
    }

    #[must_use]
    pub fn get_enchantment_level_by_id(&self, enchantment: &Identifier) -> i32 {
        self.params
            .get(loot_context_params::TOOL)
            .map_or(0, |tool| tool.get_enchantment_level(enchantment))
    }

    #[must_use]
    pub fn get_attacking_entity_enchantment_level(&self, enchantment: &Identifier) -> i32 {
        self.params
            .get(loot_context_params::ATTACKING_ENTITY)
            .and_then(|entity| entity.equipment)
            .map_or(0, |equipment| equipment.get_enchantment_level(enchantment))
    }

    #[must_use]
    pub fn get_entity(&self, target: LootContextEntity) -> Option<EntityRef<'a>> {
        let key = match target {
            LootContextEntity::This => loot_context_params::THIS_ENTITY,
            LootContextEntity::Killer => loot_context_params::ATTACKING_ENTITY,
            LootContextEntity::DirectKiller => loot_context_params::DIRECT_ATTACKING_ENTITY,
            LootContextEntity::KillerPlayer => loot_context_params::LAST_DAMAGE_PLAYER,
            LootContextEntity::Interacting => loot_context_params::INTERACTING_ENTITY,
            LootContextEntity::Target => loot_context_params::TARGET_ENTITY,
        };
        self.params.get(key)
    }
}
