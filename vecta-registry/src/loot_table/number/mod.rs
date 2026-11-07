//! Vanilla `ContextFloatProvider` / `ContextIntProvider` trees.
//!
//! Java models these as two registries of inline-codec provider types plus a
//! second registry of configured values that reference each other by key. Vecta
//! keeps the same shape: a typed enum per result family, references resolved
//! through [`FloatProviderRegistry`] / [`IntProviderRegistry`], and the two
//! Java entry points reproduced exactly:
//!
//! - `get_float` / `get_int` are the safe wrappers (`ContextFloatProvider.getFloat`
//!   and `ContextIntProvider.getInt`): any arithmetic failure or non-finite float
//!   result becomes `0.0` / `0`.
//! - `get_float_unsafe` / `get_int_unsafe` propagate [`ProviderError`], because
//!   composite providers evaluate their operands through the unsafe entry point.
//!
//! `environment_attribute` is absent: Vecta has no environment-attribute
//! foundation yet, so the type is rejected instead of being faked.

mod parse;

pub use parse::{ProviderParseError, parse_float_provider, parse_int_provider};

use std::fmt;

use rustc_hash::FxHashMap;
use vecta_math::trig;
use vecta_utils::Identifier;
use vecta_utils::nbt::NbtPath;
use vecta_utils::random::Random as _;

use super::{LootCondition, LootContext, LootScoreHolder, ScoreboardTarget, loot_context_params};
use crate::enchantment::effect::LevelBasedValue;

/// A failure while evaluating a provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    /// A provider key was not present in its registry.
    UnknownProvider(Identifier),
    /// A predicate key was not present in the predicate registry.
    UnknownPredicate(Identifier),
    /// Java `ArithmeticException`: division by zero, overflow or an undefined
    /// conversion.
    Arithmetic,
    /// `ContextFloatProvider.getFloatOrThrow` rejected a non-finite value.
    NonFinite,
    /// A weighted list without any non-zero weight.
    EmptyWeightedList,
    /// `Mth.nextInt` received a non-positive bound after overflow.
    InvalidBound,
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProvider(key) => write!(f, "unknown number provider '{key}'"),
            Self::UnknownPredicate(key) => write!(f, "unknown loot predicate '{key}'"),
            Self::Arithmetic => write!(f, "arithmetic error"),
            Self::NonFinite => write!(f, "non-finite provider value"),
            Self::EmptyWeightedList => write!(f, "weighted list has no elements"),
            Self::InvalidBound => write!(f, "invalid random bound"),
        }
    }
}

impl std::error::Error for ProviderError {}

/// A provider slot that is either a registry key or an inline provider.
#[derive(Debug, Clone)]
pub enum ProviderRef<T> {
    /// `Holder.Reference`: a key resolved through the provider registry.
    Reference(Identifier),
    /// `Holder.Direct`: an inline provider payload.
    Inline(Box<T>),
}

impl<T> ProviderRef<T> {
    /// Builds an inline reference.
    #[must_use]
    pub fn inline(value: T) -> Self {
        Self::Inline(Box::new(value))
    }
}

/// A condition slot that is either a predicate key or an inline condition.
#[derive(Debug, Clone)]
pub enum ConditionRef {
    /// `Holder.Reference` into `Registries.PREDICATE`.
    Reference(Identifier),
    /// `Holder.Direct`: an inline condition payload.
    Inline(Box<LootCondition>),
}

impl ConditionRef {
    fn test(&self, ctx: &mut LootContext<'_>) -> Result<bool, ProviderError> {
        match self {
            Self::Reference(key) => {
                let condition = super::vanilla_loot_predicates::by_key(key)
                    .ok_or_else(|| ProviderError::UnknownPredicate(key.clone()))?;
                Ok(condition.test(ctx))
            }
            Self::Inline(condition) => Ok(condition.test(ctx)),
        }
    }
}

/// One entry of a vanilla `WeightedList`.
#[derive(Debug, Clone)]
pub struct WeightedValue<T> {
    /// The weighted value.
    pub value: ProviderRef<T>,
    /// Non-negative weight, mirroring `Weighted.weight`.
    pub weight: i32,
}

impl<T> WeightedValue<T> {
    /// Builds an entry.
    #[must_use]
    pub const fn new(value: ProviderRef<T>, weight: i32) -> Self {
        Self { value, weight }
    }
}

/// Vanilla `WeightedList`: a weight table plus one bounded RNG draw.
#[derive(Debug, Clone)]
pub struct WeightedList<T> {
    /// Entries in declaration order.
    pub entries: Vec<WeightedValue<T>>,
}

impl<T> WeightedList<T> {
    /// Builds a list.
    #[must_use]
    pub const fn new(entries: Vec<WeightedValue<T>>) -> Self {
        Self { entries }
    }

    /// `WeightedList.getTotalWeight`; zero disables the selector.
    fn total_weight(&self) -> i32 {
        self.entries
            .iter()
            .fold(0i32, |total, entry| total.wrapping_add(entry.weight))
    }

    /// `WeightedList.getRandomOrThrow`: one `nextInt(totalWeight)` then a linear
    /// scan, which is what both the flat and compact Java selectors produce.
    fn select(&self, ctx: &mut LootContext<'_>) -> Result<&ProviderRef<T>, ProviderError> {
        let total = self.total_weight();
        if total <= 0 {
            return Err(ProviderError::EmptyWeightedList);
        }
        let mut selection = ctx.rng.next_i32_bounded(total);
        for entry in &self.entries {
            selection -= entry.weight;
            if selection < 0 {
                return Ok(&entry.value);
            }
        }
        Err(ProviderError::EmptyWeightedList)
    }
}

/// Vanilla `ScoreboardNameProviders` target for `minecraft:score`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScoreTarget {
    /// `this`
    This,
    /// `killer` / `attacker`
    Killer,
    /// `direct_killer` / `direct_attacker`
    DirectKiller,
    /// `killer_player` / `last_damage_player` / `attacking_player`
    KillerPlayer,
    /// `target_entity`
    Target,
    /// `interacting_entity`
    Interacting,
    /// A literal score holder name.
    Fixed(String),
}

impl ScoreTarget {
    /// Parses a vanilla scoreboard-name provider argument.
    #[must_use]
    pub fn parse(name: &str) -> Self {
        match name {
            "this" => Self::This,
            "killer" | "attacker" => Self::Killer,
            "direct_killer" | "direct_attacker" => Self::DirectKiller,
            "killer_player" | "last_damage_player" | "attacking_player" => Self::KillerPlayer,
            "target_entity" => Self::Target,
            "interacting_entity" => Self::Interacting,
            other => Self::Fixed(other.to_owned()),
        }
    }

    const fn as_legacy(&self) -> Option<ScoreboardTarget> {
        Some(match self {
            Self::This => ScoreboardTarget::This,
            Self::Killer => ScoreboardTarget::Killer,
            Self::DirectKiller => ScoreboardTarget::DirectKiller,
            Self::KillerPlayer => ScoreboardTarget::KillerPlayer,
            Self::Target => ScoreboardTarget::Target,
            Self::Interacting => ScoreboardTarget::Interacting,
            Self::Fixed(_) => return None,
        })
    }

    fn read(&self, ctx: &mut LootContext<'_>, objective: &str) -> Option<i32> {
        if let Some(legacy) = self.as_legacy() {
            return legacy.get_score(ctx, objective);
        }
        let Self::Fixed(name) = self else {
            return None;
        };
        let data = ctx.data?;
        data.score(LootScoreHolder::Name(name), objective)
    }
}

/// A `ContextFloatProvider` implementation.
#[derive(Debug, Clone)]
pub enum FloatProvider {
    /// `minecraft:constant`
    Constant(f32),
    /// `minecraft:abs`
    Abs(ProviderRef<Self>),
    /// `minecraft:avg`
    Average(Vec<ProviderRef<Self>>),
    /// `minecraft:ceil`
    Ceil(ProviderRef<Self>),
    /// `minecraft:conditional`
    Conditional {
        /// Branch predicate.
        condition: ConditionRef,
        /// Value when the predicate passes.
        on_true: ProviderRef<Self>,
        /// Value when the predicate fails; `on_false` defaults to 0.
        on_false: ProviderRef<Self>,
    },
    /// `minecraft:cos`
    Cos(ProviderRef<Self>),
    /// `minecraft:sub`
    Sub(ProviderRef<Self>, ProviderRef<Self>),
    /// `minecraft:enchantment_level`
    EnchantmentLevel(&'static LevelBasedValue),
    /// `minecraft:floor`
    Floor(ProviderRef<Self>),
    /// `minecraft:from_int`
    FromInt(ProviderRef<IntProvider>),
    /// `minecraft:length`
    Length(Vec<ProviderRef<Self>>),
    /// `minecraft:max`
    Max(Vec<ProviderRef<Self>>),
    /// `minecraft:min`
    Min(Vec<ProviderRef<Self>>),
    /// `minecraft:mod`
    Mod(ProviderRef<Self>, ProviderRef<Self>),
    /// `minecraft:negate`
    Negate(ProviderRef<Self>),
    /// `minecraft:number_dispatcher`
    NumberDispatcher {
        /// Cases tested in order.
        cases: Vec<(ConditionRef, ProviderRef<Self>)>,
        /// Value when no case matches.
        default: ProviderRef<Self>,
    },
    /// `minecraft:pow`
    Pow {
        /// Base operand.
        base: ProviderRef<Self>,
        /// Exponent operand.
        exponent: ProviderRef<Self>,
    },
    /// `minecraft:mul`
    Mul(Vec<ProviderRef<Self>>),
    /// `minecraft:div`
    Div(ProviderRef<Self>, ProviderRef<Self>),
    /// `minecraft:round`
    Round(ProviderRef<Self>),
    /// `minecraft:sin`
    Sin(ProviderRef<Self>),
    /// `minecraft:sqrt`
    Sqrt(ProviderRef<Self>),
    /// `minecraft:storage`
    Storage {
        /// Command storage id.
        storage: Identifier,
        /// Numeric `NbtPath`.
        path: NbtPath,
        /// Value used when the path does not resolve to one number.
        fallback: ProviderRef<Self>,
    },
    /// `minecraft:add`
    Add(Vec<ProviderRef<Self>>),
    /// `minecraft:truncate`
    Truncate(ProviderRef<Self>),
    /// `minecraft:uniform`
    Uniform {
        /// Inclusive lower bound.
        min: ProviderRef<Self>,
        /// Inclusive upper bound.
        max: ProviderRef<Self>,
    },
    /// `minecraft:weighted_list`
    WeightedList(WeightedList<Self>),
}

/// A `ContextIntProvider` implementation.
#[derive(Debug, Clone)]
pub enum IntProvider {
    /// `minecraft:abs`
    Abs(ProviderRef<Self>),
    /// `minecraft:avg`
    Average(Vec<ProviderRef<Self>>),
    /// `minecraft:binomial`
    Binomial {
        /// Number of trials.
        n: ProviderRef<Self>,
        /// Success probability.
        p: ProviderRef<FloatProvider>,
    },
    /// `minecraft:conditional`
    Conditional {
        /// Branch predicate.
        condition: ConditionRef,
        /// Value when the predicate passes.
        on_true: ProviderRef<Self>,
        /// Value when the predicate fails.
        on_false: ProviderRef<Self>,
    },
    /// `minecraft:constant`
    Constant(i32),
    /// `minecraft:sub`
    Sub(ProviderRef<Self>, ProviderRef<Self>),
    /// `minecraft:from_float`
    FromFloat(ProviderRef<FloatProvider>),
    /// `minecraft:max`
    Max(Vec<ProviderRef<Self>>),
    /// `minecraft:min`
    Min(Vec<ProviderRef<Self>>),
    /// `minecraft:floor_mod`
    FloorMod(ProviderRef<Self>, ProviderRef<Self>),
    /// `minecraft:floor_div`
    FloorDiv(ProviderRef<Self>, ProviderRef<Self>),
    /// `minecraft:mod`
    Mod(ProviderRef<Self>, ProviderRef<Self>),
    /// `minecraft:div`
    Div(ProviderRef<Self>, ProviderRef<Self>),
    /// `minecraft:negate`
    Negate(ProviderRef<Self>),
    /// `minecraft:number_dispatcher`
    NumberDispatcher {
        /// Cases tested in order.
        cases: Vec<(ConditionRef, ProviderRef<Self>)>,
        /// Value when no case matches.
        default: ProviderRef<Self>,
    },
    /// `minecraft:pow`
    Pow {
        /// Base operand.
        base: ProviderRef<Self>,
        /// Exponent operand.
        exponent: ProviderRef<Self>,
    },
    /// `minecraft:mul`
    Mul(Vec<ProviderRef<Self>>),
    /// `minecraft:score`
    Score {
        /// Score holder selector.
        target: ScoreTarget,
        /// Objective name.
        score: String,
        /// Value used when the score does not exist.
        fallback: ProviderRef<Self>,
    },
    /// `minecraft:storage`
    Storage {
        /// Command storage id.
        storage: Identifier,
        /// Numeric `NbtPath`.
        path: NbtPath,
        /// Value used when the path does not resolve to one number.
        fallback: ProviderRef<Self>,
    },
    /// `minecraft:add`
    Add(Vec<ProviderRef<Self>>),
    /// `minecraft:uniform`
    Uniform {
        /// Inclusive lower bound.
        min: ProviderRef<Self>,
        /// Inclusive upper bound.
        max: ProviderRef<Self>,
    },
    /// `minecraft:weighted_list`
    WeightedList(WeightedList<Self>),
}

/// Registry of configured `ContextFloatProvider` values.
#[derive(Default)]
pub struct FloatProviderRegistry {
    entries: Vec<(Identifier, Box<FloatProvider>)>,
    by_key: FxHashMap<Identifier, usize>,
    frozen: bool,
}

/// Registry of configured `ContextIntProvider` values.
#[derive(Default)]
pub struct IntProviderRegistry {
    entries: Vec<(Identifier, Box<IntProvider>)>,
    by_key: FxHashMap<Identifier, usize>,
    frozen: bool,
}

macro_rules! provider_registry {
    ($registry:ident, $provider:ty) => {
        impl $registry {
            /// Creates an empty registry that still accepts registrations.
            #[must_use]
            pub fn new() -> Self {
                Self {
                    entries: Vec::new(),
                    by_key: FxHashMap::default(),
                    frozen: false,
                }
            }

            /// Registers one configured value.
            ///
            /// # Panics
            ///
            /// Panics on a duplicate key or after the registry has been frozen,
            /// matching the other Vecta registries.
            pub fn register(&mut self, key: Identifier, provider: $provider) {
                assert!(
                    !self.frozen,
                    "cannot register a number provider after the registry is frozen"
                );
                assert!(
                    !self.by_key.contains_key(&key),
                    "duplicate number provider key {key}"
                );
                self.by_key.insert(key.clone(), self.entries.len());
                self.entries.push((key, Box::new(provider)));
            }

            /// Looks up a configured value.
            #[must_use]
            pub fn by_key(&self, key: &Identifier) -> Option<&$provider> {
                self.by_key
                    .get(key)
                    .map(|index| self.entries[*index].1.as_ref())
            }

            /// Freezes the registry.
            pub const fn freeze(&mut self) {
                self.frozen = true;
            }

            /// Returns the registered key/provider pairs.
            pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &$provider)> {
                self.entries
                    .iter()
                    .map(|(key, provider)| (key, provider.as_ref()))
            }

            /// Returns the number of registered values.
            #[must_use]
            pub const fn len(&self) -> usize {
                self.entries.len()
            }

            /// Returns whether the registry is empty.
            #[must_use]
            pub const fn is_empty(&self) -> bool {
                self.entries.is_empty()
            }
        }
    };
}

provider_registry!(FloatProviderRegistry, FloatProvider);
provider_registry!(IntProviderRegistry, IntProvider);

fn resolve_float(reference: &ProviderRef<FloatProvider>) -> Result<&FloatProvider, ProviderError> {
    match reference {
        ProviderRef::Reference(key) => crate::REGISTRY
            .context_float_providers
            .by_key(key)
            .ok_or_else(|| ProviderError::UnknownProvider(key.clone())),
        ProviderRef::Inline(provider) => Ok(provider),
    }
}

fn resolve_int(reference: &ProviderRef<IntProvider>) -> Result<&IntProvider, ProviderError> {
    match reference {
        ProviderRef::Reference(key) => crate::REGISTRY
            .context_int_providers
            .by_key(key)
            .ok_or_else(|| ProviderError::UnknownProvider(key.clone())),
        ProviderRef::Inline(provider) => Ok(provider),
    }
}

/// `Math.round(float)` — NaN becomes 0 and infinities saturate.
fn java_round(value: f32) -> i32 {
    if value.is_nan() {
        return 0;
    }
    (value + 0.5).floor() as i32
}

/// `Mth.ceil(float)` — `(int) Math.ceil`, so NaN becomes 0 and huge values
/// saturate at the int bounds.
const fn java_ceil(value: f32) -> i32 {
    if value.is_nan() {
        return 0;
    }
    value.ceil() as i32
}

/// `(int)` narrowing of a `long`, as Java performs in `longToIntSafe`.
fn long_to_int_safe(value: i64) -> Result<i32, ProviderError> {
    i32::try_from(value).map_err(|_| ProviderError::Arithmetic)
}

/// `ContextIntProvider.floatToIntSafe`.
fn float_to_int_safe(value: f32) -> Result<i32, ProviderError> {
    if !value.is_finite() {
        return Err(ProviderError::Arithmetic);
    }
    // Java casts float to long first (saturating), then checks the int range.
    let widened = value as i64;
    long_to_int_safe(widened)
}

/// `Math.absExact`.
fn abs_exact(value: i32) -> Result<i32, ProviderError> {
    value.checked_abs().ok_or(ProviderError::Arithmetic)
}

/// `Math.negateExact`.
fn negate_exact(value: i32) -> Result<i32, ProviderError> {
    value.checked_neg().ok_or(ProviderError::Arithmetic)
}

/// `Math.floorMod`: the result takes the sign of the divisor, so this is not
/// Rust's Euclidean remainder.
const fn floor_mod(left: i32, right: i32) -> Result<i32, ProviderError> {
    if right == 0 {
        return Err(ProviderError::Arithmetic);
    }
    let remainder = left.wrapping_rem(right);
    if remainder != 0 && (remainder < 0) != (right < 0) {
        Ok(remainder + right)
    } else {
        Ok(remainder)
    }
}

/// `Math.floorDivExact`: rounds towards negative infinity, so this is not Rust's
/// Euclidean division, plus the overflow check.
fn floor_div_exact(left: i32, right: i32) -> Result<i32, ProviderError> {
    let quotient = left.checked_div(right).ok_or(ProviderError::Arithmetic)?;
    if left % right != 0 && (left < 0) != (right < 0) {
        Ok(quotient - 1)
    } else {
        Ok(quotient)
    }
}

/// `Math.max(float, float)`: NaN wins and `0.0` beats `-0.0`, unlike `f32::max`.
const fn java_max(left: f32, right: f32) -> f32 {
    if left.is_nan() || right.is_nan() {
        f32::NAN
    } else if left.total_cmp(&right).is_ge() {
        left
    } else {
        right
    }
}

/// `Math.min(float, float)`: NaN wins and `-0.0` beats `0.0`, unlike `f32::min`.
const fn java_min(left: f32, right: f32) -> f32 {
    if left.is_nan() || right.is_nan() {
        f32::NAN
    } else if left.total_cmp(&right).is_le() {
        left
    } else {
        right
    }
}

/// `Math.pow`: a base of magnitude one with a non-finite exponent is NaN, while
/// `powf` follows C and returns one.
fn java_pow(base: f64, exponent: f64) -> f64 {
    if base.abs().total_cmp(&1.0).is_eq() && !exponent.is_finite() {
        f64::NAN
    } else {
        base.powf(exponent)
    }
}

/// `Math.powExact(int, int)`.
///
/// The oracle run against JDK 25 shows that every negative exponent throws, and
/// that the result must fit in an `int`.
fn pow_exact(base: i32, exponent: i32) -> Result<i32, ProviderError> {
    if exponent < 0 {
        return Err(ProviderError::Arithmetic);
    }
    let mut result: i32 = 1;
    let mut remaining = exponent;
    let mut factor = base;
    while remaining > 0 {
        if remaining & 1 == 1 {
            result = result
                .checked_mul(factor)
                .ok_or(ProviderError::Arithmetic)?;
        }
        remaining >>= 1;
        if remaining > 0 {
            factor = factor
                .checked_mul(factor)
                .ok_or(ProviderError::Arithmetic)?;
        }
    }
    Ok(result)
}

/// `Mth.nextInt(random, min, max)`.
fn next_int_between(ctx: &mut LootContext<'_>, min: i32, max: i32) -> Result<i32, ProviderError> {
    if min >= max {
        return Ok(min);
    }
    let bound = max.wrapping_sub(min).wrapping_add(1);
    if bound <= 0 {
        return Err(ProviderError::InvalidBound);
    }
    Ok(ctx.rng.next_i32_bounded(bound).wrapping_add(min))
}

/// `Mth.nextFloat(random, min, max)`.
fn next_float_between(ctx: &mut LootContext<'_>, min: f32, max: f32) -> f32 {
    if min >= max {
        return min;
    }
    ctx.rng.next_f32() * (max - min) + min
}

fn sum_floats(
    providers: &[ProviderRef<FloatProvider>],
    ctx: &mut LootContext<'_>,
) -> Result<f32, ProviderError> {
    let mut total = 0.0f32;
    for provider in providers {
        total += resolve_float(provider)?.get_float_unsafe(ctx)?;
    }
    Ok(total)
}

fn product_floats(
    providers: &[ProviderRef<FloatProvider>],
    ctx: &mut LootContext<'_>,
) -> Result<f32, ProviderError> {
    let mut total = 1.0f32;
    for provider in providers {
        total *= resolve_float(provider)?.get_float_unsafe(ctx)?;
    }
    Ok(total)
}

fn sum_ints(
    providers: &[ProviderRef<IntProvider>],
    ctx: &mut LootContext<'_>,
) -> Result<i64, ProviderError> {
    let mut total = 0i64;
    for provider in providers {
        total += i64::from(resolve_int(provider)?.get_int_unsafe(ctx)?);
    }
    Ok(total)
}

fn product_ints(
    providers: &[ProviderRef<IntProvider>],
    ctx: &mut LootContext<'_>,
) -> Result<i64, ProviderError> {
    let mut total = 1i64;
    for provider in providers {
        total = total.wrapping_mul(i64::from(resolve_int(provider)?.get_int_unsafe(ctx)?));
    }
    Ok(total)
}

fn dispatch_case<'a, T>(
    cases: &'a [(ConditionRef, ProviderRef<T>)],
    ctx: &mut LootContext<'_>,
) -> Result<Option<&'a ProviderRef<T>>, ProviderError> {
    for (condition, value) in cases {
        if condition.test(ctx)? {
            return Ok(Some(value));
        }
    }
    Ok(None)
}

impl FloatProvider {
    /// `ContextFloatProvider.getFloat`: any failure or non-finite result is 0.
    #[must_use]
    pub fn get_float(&self, ctx: &mut LootContext<'_>) -> f32 {
        match self.get_float_unsafe(ctx) {
            Ok(value) if value.is_finite() => value,
            _ => 0.0,
        }
    }

    /// `ContextFloatProvider.getFloatOrThrow` for a registry entry.
    pub fn get_float_or_throw(&self, ctx: &mut LootContext<'_>) -> Result<f32, ProviderError> {
        let value = self.get_float_unsafe(ctx)?;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(ProviderError::NonFinite)
        }
    }

    /// `ContextFloatProvider.getFloatUnsafe`.
    pub fn get_float_unsafe(&self, ctx: &mut LootContext<'_>) -> Result<f32, ProviderError> {
        match self {
            Self::Constant(value) => Ok(*value),
            Self::Abs(input) => Ok(resolve_float(input)?.get_float_unsafe(ctx)?.abs()),
            Self::Average(inputs) => {
                let mut sum = 0.0f32;
                let mut count = 0i32;
                for input in inputs {
                    sum += resolve_float(input)?.get_float_unsafe(ctx)?;
                    count += 1;
                }
                Ok(sum / count as f32)
            }
            Self::Ceil(input) => Ok(java_ceil(resolve_float(input)?.get_float_unsafe(ctx)?) as f32),
            Self::Conditional {
                condition,
                on_true,
                on_false,
            } => {
                let selected = if condition.test(ctx)? {
                    on_true
                } else {
                    on_false
                };
                resolve_float(selected)?.get_float_unsafe(ctx)
            }
            Self::Cos(input) => Ok(trig::cos(f64::from(
                resolve_float(input)?.get_float_unsafe(ctx)?,
            ))),
            Self::Sub(left, right) => {
                let left = resolve_float(left)?.get_float_unsafe(ctx)?;
                Ok(left - resolve_float(right)?.get_float_unsafe(ctx)?)
            }
            Self::EnchantmentLevel(amount) => {
                let level = ctx
                    .params
                    .get(loot_context_params::ENCHANTMENT_LEVEL)
                    .unwrap_or(0);
                Ok(amount.calculate(level))
            }
            Self::Floor(input) => Ok(resolve_float(input)?.get_float_unsafe(ctx)?.floor()),
            Self::FromInt(input) => Ok(resolve_int(input)?.get_int_unsafe(ctx)? as f32),
            Self::Length(inputs) => {
                let mut sum_of_squares = 0.0f32;
                for input in inputs {
                    let value = resolve_float(input)?.get_float_unsafe(ctx)?;
                    sum_of_squares += value * value;
                }
                Ok(sum_of_squares.sqrt())
            }
            Self::Max(inputs) => {
                let mut value = -f32::MAX;
                for input in inputs {
                    value = java_max(value, resolve_float(input)?.get_float_unsafe(ctx)?);
                }
                Ok(value)
            }
            Self::Min(inputs) => {
                let mut value = f32::MAX;
                for input in inputs {
                    value = java_min(value, resolve_float(input)?.get_float_unsafe(ctx)?);
                }
                Ok(value)
            }
            Self::Mod(left, right) => {
                // Vanilla evaluates the divisor first.
                let divisor = resolve_float(right)?.get_float_unsafe(ctx)?;
                if divisor == 0.0 {
                    return Ok(f32::NAN);
                }
                Ok(resolve_float(left)?.get_float_unsafe(ctx)? % divisor)
            }
            Self::Negate(input) => Ok(-resolve_float(input)?.get_float_unsafe(ctx)?),
            Self::NumberDispatcher { cases, default } => {
                let selected = dispatch_case(cases, ctx)?.unwrap_or(default);
                resolve_float(selected)?.get_float_unsafe(ctx)
            }
            Self::Pow { base, exponent } => {
                let base = resolve_float(base)?.get_float_unsafe(ctx)?;
                let exponent = resolve_float(exponent)?.get_float_unsafe(ctx)?;
                if base == 0.0 && exponent == 0.0 {
                    return Ok(f32::NAN);
                }
                Ok(java_pow(f64::from(base), f64::from(exponent)) as f32)
            }
            Self::Mul(inputs) => product_floats(inputs, ctx),
            Self::Div(left, right) => {
                let left = resolve_float(left)?.get_float_unsafe(ctx)?;
                Ok(left / resolve_float(right)?.get_float_unsafe(ctx)?)
            }
            Self::Round(input) => {
                Ok(java_round(resolve_float(input)?.get_float_unsafe(ctx)?) as f32)
            }
            Self::Sin(input) => Ok(trig::sin(f64::from(
                resolve_float(input)?.get_float_unsafe(ctx)?,
            ))),
            Self::Sqrt(input) => Ok(resolve_float(input)?.get_float_unsafe(ctx)?.sqrt()),
            Self::Storage {
                storage,
                path,
                fallback,
            } => {
                if let Some(value) = ctx.data.and_then(|data| data.storage_float(storage, path)) {
                    return Ok(value);
                }
                resolve_float(fallback)?.get_float_unsafe(ctx)
            }
            Self::Add(inputs) => sum_floats(inputs, ctx),
            Self::Truncate(input) => {
                let value = resolve_float(input)?.get_float_unsafe(ctx)?;
                Ok(if value > 0.0 {
                    value.floor()
                } else {
                    value.ceil()
                })
            }
            Self::Uniform { min, max } => {
                let min = resolve_float(min)?.get_float_unsafe(ctx)?;
                let max = resolve_float(max)?.get_float_unsafe(ctx)?;
                Ok(next_float_between(ctx, min, max))
            }
            Self::WeightedList(list) => {
                let selected = list.select(ctx)?;
                resolve_float(selected)?.get_float_unsafe(ctx)
            }
        }
    }
}

impl IntProvider {
    /// `ContextIntProvider.getInt`: any arithmetic failure is 0.
    #[must_use]
    pub fn get_int(&self, ctx: &mut LootContext<'_>) -> i32 {
        self.get_int_unsafe(ctx).unwrap_or(0)
    }

    /// `ContextIntProvider.getIntUnsafe`.
    pub fn get_int_unsafe(&self, ctx: &mut LootContext<'_>) -> Result<i32, ProviderError> {
        match self {
            Self::Abs(input) => abs_exact(resolve_int(input)?.get_int_unsafe(ctx)?),
            Self::Average(inputs) => {
                let mut sum = 0i64;
                let mut count = 0i64;
                for input in inputs {
                    sum += i64::from(resolve_int(input)?.get_int_unsafe(ctx)?);
                    count += 1;
                }
                if count == 0 {
                    return Err(ProviderError::Arithmetic);
                }
                long_to_int_safe(sum / count)
            }
            Self::Binomial { n, p } => {
                let n = resolve_int(n)?.get_int_unsafe(ctx)?;
                let probability = resolve_float(p)?.get_float_or_throw(ctx)?;
                let mut result = 0;
                for _ in 0..n {
                    if ctx.rng.next_f32() < probability {
                        result += 1;
                    }
                }
                Ok(result)
            }
            Self::Conditional {
                condition,
                on_true,
                on_false,
            } => {
                let selected = if condition.test(ctx)? {
                    on_true
                } else {
                    on_false
                };
                resolve_int(selected)?.get_int_unsafe(ctx)
            }
            Self::Constant(value) => Ok(*value),
            Self::Sub(left, right) => {
                let left = resolve_int(left)?.get_int_unsafe(ctx)?;
                left.checked_sub(resolve_int(right)?.get_int_unsafe(ctx)?)
                    .ok_or(ProviderError::Arithmetic)
            }
            Self::FromFloat(input) => {
                float_to_int_safe(resolve_float(input)?.get_float_unsafe(ctx)?)
            }
            Self::Max(inputs) => {
                let mut value = i32::MIN;
                for input in inputs {
                    value = value.max(resolve_int(input)?.get_int_unsafe(ctx)?);
                }
                Ok(value)
            }
            Self::Min(inputs) => {
                let mut value = i32::MAX;
                for input in inputs {
                    value = value.min(resolve_int(input)?.get_int_unsafe(ctx)?);
                }
                Ok(value)
            }
            Self::FloorMod(left, right) => {
                let left = resolve_int(left)?.get_int_unsafe(ctx)?;
                floor_mod(left, resolve_int(right)?.get_int_unsafe(ctx)?)
            }
            Self::FloorDiv(left, right) => {
                let left = resolve_int(left)?.get_int_unsafe(ctx)?;
                floor_div_exact(left, resolve_int(right)?.get_int_unsafe(ctx)?)
            }
            Self::Mod(left, right) => {
                let left = resolve_int(left)?.get_int_unsafe(ctx)?;
                let right = resolve_int(right)?.get_int_unsafe(ctx)?;
                if right == 0 {
                    return Err(ProviderError::Arithmetic);
                }
                Ok(left.wrapping_rem(right))
            }
            Self::Div(left, right) => {
                let left = resolve_int(left)?.get_int_unsafe(ctx)?;
                let right = resolve_int(right)?.get_int_unsafe(ctx)?;
                if right == 0 {
                    return Err(ProviderError::Arithmetic);
                }
                Ok(left.wrapping_div(right))
            }
            Self::Negate(input) => negate_exact(resolve_int(input)?.get_int_unsafe(ctx)?),
            Self::NumberDispatcher { cases, default } => {
                let selected = dispatch_case(cases, ctx)?.unwrap_or(default);
                resolve_int(selected)?.get_int_unsafe(ctx)
            }
            Self::Pow { base, exponent } => {
                let base = resolve_int(base)?.get_int_unsafe(ctx)?;
                let exponent = resolve_int(exponent)?.get_int_unsafe(ctx)?;
                if base == 0 && exponent == 0 {
                    return Err(ProviderError::Arithmetic);
                }
                pow_exact(base, exponent)
            }
            Self::Mul(inputs) => long_to_int_safe(product_ints(inputs, ctx)?),
            Self::Score {
                target,
                score,
                fallback,
            } => {
                if let Some(value) = target.read(ctx, score) {
                    return Ok(value);
                }
                resolve_int(fallback)?.get_int_unsafe(ctx)
            }
            Self::Storage {
                storage,
                path,
                fallback,
            } => {
                if let Some(value) = ctx.data.and_then(|data| data.storage_int(storage, path)) {
                    return Ok(value);
                }
                resolve_int(fallback)?.get_int_unsafe(ctx)
            }
            Self::Add(inputs) => long_to_int_safe(sum_ints(inputs, ctx)?),
            Self::Uniform { min, max } => {
                let min = resolve_int(min)?.get_int_unsafe(ctx)?;
                let max = resolve_int(max)?.get_int_unsafe(ctx)?;
                next_int_between(ctx, min, max)
            }
            Self::WeightedList(list) => {
                let selected = list.select(ctx)?;
                resolve_int(selected)?.get_int_unsafe(ctx)
            }
        }
    }
}

#[cfg(test)]
mod tests;
