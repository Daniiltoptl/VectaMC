//! Runtime parsing of inline number-provider payloads.
//!
//! Vanilla accepts a provider either as a registry key (a string) or as an inline
//! object with a `type` field. Commands use exactly that shape, so the parser is
//! the runtime half of the same model the build generator emits for configured
//! values.
//!
//! `environment_attribute` and `enchantment_level` are rejected: the first needs
//! an environment-attribute foundation Vecta does not have, and the second needs
//! an owned `LevelBasedValue` that the enchantment registry stores as `'static`.

use std::fmt;
use std::str::FromStr;

use simdnbt::owned::{NbtCompound, NbtTag};
use vecta_utils::Identifier;
use vecta_utils::nbt::{nbt_list_values, parse_nbt_path};

use super::{
    ConditionRef, FloatProvider, IntProvider, ProviderRef, ScoreTarget, WeightedList, WeightedValue,
};

/// A malformed or unsupported provider payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderParseError {
    /// The payload has no `type` field.
    MissingType,
    /// The `type` field names a provider this build cannot evaluate.
    UnsupportedType(String),
    /// A required field is absent.
    MissingField(&'static str),
    /// A field has the wrong tag or value type.
    InvalidField(&'static str),
    /// An identifier or NBT path did not parse.
    InvalidValue(&'static str),
}

impl fmt::Display for ProviderParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingType => write!(f, "number provider payload has no type"),
            Self::UnsupportedType(provider_type) => {
                write!(f, "unsupported number provider type '{provider_type}'")
            }
            Self::MissingField(field) => write!(f, "number provider is missing '{field}'"),
            Self::InvalidField(field) => write!(f, "number provider field '{field}' is invalid"),
            Self::InvalidValue(field) => write!(f, "number provider field '{field}' did not parse"),
        }
    }
}

impl std::error::Error for ProviderParseError {}

/// Parses a `ContextFloatProvider` payload.
pub fn parse_float_provider(nbt: &NbtCompound) -> Result<FloatProvider, ProviderParseError> {
    let provider_type = provider_type(nbt)?;
    let provider = match provider_type.as_str() {
        "minecraft:constant" => FloatProvider::Constant(float_field(nbt, "value")?),
        "minecraft:abs" => FloatProvider::Abs(float_ref(nbt, "input")?),
        "minecraft:avg" => FloatProvider::Average(float_refs(nbt, "inputs")?),
        "minecraft:ceil" => FloatProvider::Ceil(float_ref(nbt, "input")?),
        "minecraft:conditional" => FloatProvider::Conditional {
            condition: condition_ref(nbt, "condition")?,
            on_true: float_ref(nbt, "on_true")?,
            on_false: optional_float_ref(nbt, "on_false")?,
        },
        "minecraft:cos" => FloatProvider::Cos(float_ref(nbt, "input")?),
        "minecraft:sub" => FloatProvider::Sub(float_ref(nbt, "left")?, float_ref(nbt, "right")?),
        "minecraft:floor" => FloatProvider::Floor(float_ref(nbt, "input")?),
        "minecraft:from_int" => FloatProvider::FromInt(int_ref(nbt, "input")?),
        "minecraft:length" => FloatProvider::Length(float_refs(nbt, "inputs")?),
        "minecraft:max" => FloatProvider::Max(float_refs(nbt, "inputs")?),
        "minecraft:min" => FloatProvider::Min(float_refs(nbt, "inputs")?),
        "minecraft:mod" => FloatProvider::Mod(float_ref(nbt, "left")?, float_ref(nbt, "right")?),
        "minecraft:negate" => FloatProvider::Negate(float_ref(nbt, "input")?),
        "minecraft:number_dispatcher" => {
            let (cases, default) = dispatcher(
                nbt,
                parse_float_provider,
                |value| FloatProvider::Constant(value as f32),
                || FloatProvider::Constant(0.0),
            )?;
            FloatProvider::NumberDispatcher { cases, default }
        }
        "minecraft:pow" => FloatProvider::Pow {
            base: float_ref(nbt, "base")?,
            exponent: float_ref(nbt, "exponent")?,
        },
        "minecraft:mul" => FloatProvider::Mul(float_refs(nbt, "inputs")?),
        "minecraft:div" => FloatProvider::Div(float_ref(nbt, "left")?, float_ref(nbt, "right")?),
        "minecraft:round" => FloatProvider::Round(float_ref(nbt, "input")?),
        "minecraft:sin" => FloatProvider::Sin(float_ref(nbt, "input")?),
        "minecraft:sqrt" => FloatProvider::Sqrt(float_ref(nbt, "input")?),
        "minecraft:storage" => {
            let (storage, path) = storage_access(nbt)?;
            FloatProvider::Storage {
                storage,
                path,
                fallback: optional_float_ref(nbt, "fallback")?,
            }
        }
        "minecraft:add" => FloatProvider::Add(float_refs(nbt, "inputs")?),
        "minecraft:truncate" => FloatProvider::Truncate(float_ref(nbt, "input")?),
        "minecraft:uniform" => FloatProvider::Uniform {
            min: float_ref(nbt, "min")?,
            max: float_ref(nbt, "max")?,
        },
        "minecraft:weighted_list" => {
            FloatProvider::WeightedList(weighted_list(nbt, parse_float_provider, |value| {
                FloatProvider::Constant(value as f32)
            })?)
        }
        other => return Err(ProviderParseError::UnsupportedType(other.to_owned())),
    };
    Ok(provider)
}

/// Parses a `ContextIntProvider` payload.
pub fn parse_int_provider(nbt: &NbtCompound) -> Result<IntProvider, ProviderParseError> {
    let provider_type = provider_type(nbt)?;
    let provider = match provider_type.as_str() {
        "minecraft:abs" => IntProvider::Abs(int_ref(nbt, "input")?),
        "minecraft:avg" => IntProvider::Average(int_refs(nbt, "inputs")?),
        "minecraft:binomial" => IntProvider::Binomial {
            n: int_ref(nbt, "n")?,
            p: float_ref(nbt, "p")?,
        },
        "minecraft:conditional" => IntProvider::Conditional {
            condition: condition_ref(nbt, "condition")?,
            on_true: int_ref(nbt, "on_true")?,
            on_false: optional_int_ref(nbt, "on_false")?,
        },
        "minecraft:constant" => IntProvider::Constant(int_field(nbt, "value")?),
        "minecraft:sub" => IntProvider::Sub(int_ref(nbt, "left")?, int_ref(nbt, "right")?),
        "minecraft:from_float" => IntProvider::FromFloat(float_ref(nbt, "input")?),
        "minecraft:max" => IntProvider::Max(int_refs(nbt, "inputs")?),
        "minecraft:min" => IntProvider::Min(int_refs(nbt, "inputs")?),
        "minecraft:floor_mod" => {
            IntProvider::FloorMod(int_ref(nbt, "left")?, int_ref(nbt, "right")?)
        }
        "minecraft:floor_div" => {
            IntProvider::FloorDiv(int_ref(nbt, "left")?, int_ref(nbt, "right")?)
        }
        "minecraft:mod" => IntProvider::Mod(int_ref(nbt, "left")?, int_ref(nbt, "right")?),
        "minecraft:div" => IntProvider::Div(int_ref(nbt, "left")?, int_ref(nbt, "right")?),
        "minecraft:negate" => IntProvider::Negate(int_ref(nbt, "input")?),
        "minecraft:number_dispatcher" => {
            let (cases, default) = dispatcher(
                nbt,
                parse_int_provider,
                |value| IntProvider::Constant(value as i32),
                || IntProvider::Constant(0),
            )?;
            IntProvider::NumberDispatcher { cases, default }
        }
        "minecraft:pow" => IntProvider::Pow {
            base: int_ref(nbt, "base")?,
            exponent: int_ref(nbt, "exponent")?,
        },
        "minecraft:mul" => IntProvider::Mul(int_refs(nbt, "inputs")?),
        "minecraft:score" => {
            let target = nbt
                .string("target")
                .ok_or(ProviderParseError::MissingField("target"))?;
            let target = ScoreTarget::parse(&target.to_string());
            let score = nbt
                .string("score")
                .ok_or(ProviderParseError::MissingField("score"))?
                .to_string();
            IntProvider::Score {
                target,
                score,
                fallback: optional_int_ref(nbt, "fallback")?,
            }
        }
        "minecraft:storage" => {
            let (storage, path) = storage_access(nbt)?;
            IntProvider::Storage {
                storage,
                path,
                fallback: optional_int_ref(nbt, "fallback")?,
            }
        }
        "minecraft:add" => IntProvider::Add(int_refs(nbt, "inputs")?),
        "minecraft:uniform" => IntProvider::Uniform {
            min: int_ref(nbt, "min")?,
            max: int_ref(nbt, "max")?,
        },
        "minecraft:weighted_list" => {
            IntProvider::WeightedList(weighted_list(nbt, parse_int_provider, |value| {
                IntProvider::Constant(value as i32)
            })?)
        }
        other => return Err(ProviderParseError::UnsupportedType(other.to_owned())),
    };
    Ok(provider)
}

fn provider_type(nbt: &NbtCompound) -> Result<String, ProviderParseError> {
    let provider_type = nbt.string("type").ok_or(ProviderParseError::MissingType)?;
    Ok(provider_type.to_string())
}

fn tag<'a>(nbt: &'a NbtCompound, field: &'static str) -> Result<&'a NbtTag, ProviderParseError> {
    nbt.get(field)
        .ok_or(ProviderParseError::MissingField(field))
}

fn float_field(nbt: &NbtCompound, field: &'static str) -> Result<f32, ProviderParseError> {
    tag(nbt, field)?
        .double()
        .map(|value| value as f32)
        .ok_or(ProviderParseError::InvalidField(field))
}

fn int_field(nbt: &NbtCompound, field: &'static str) -> Result<i32, ProviderParseError> {
    tag(nbt, field)?
        .int()
        .ok_or(ProviderParseError::InvalidField(field))
}

/// Reads one provider slot: a registry key or an inline payload.
fn provider_ref<T>(
    nbt: &NbtCompound,
    field: &'static str,
    parse: impl Fn(&NbtCompound) -> Result<T, ProviderParseError>,
    from_number: impl Fn(f64) -> T,
) -> Result<ProviderRef<T>, ProviderParseError> {
    match tag(nbt, field)? {
        NbtTag::String(key) => Ok(ProviderRef::Reference(identifier(&key.to_string())?)),
        NbtTag::Compound(compound) => Ok(ProviderRef::inline(parse(compound)?)),
        // `ConstantValue.INLINE_CODEC`: any numeric tag is an inline constant.
        NbtTag::Byte(value) => Ok(ProviderRef::inline(from_number(f64::from(*value)))),
        NbtTag::Short(value) => Ok(ProviderRef::inline(from_number(f64::from(*value)))),
        NbtTag::Int(value) => Ok(ProviderRef::inline(from_number(f64::from(*value)))),
        NbtTag::Long(value) => Ok(ProviderRef::inline(from_number(*value as f64))),
        NbtTag::Float(value) => Ok(ProviderRef::inline(from_number(f64::from(*value)))),
        NbtTag::Double(value) => Ok(ProviderRef::inline(from_number(*value))),
        _ => Err(ProviderParseError::InvalidField(field)),
    }
}

fn optional_ref<T>(
    nbt: &NbtCompound,
    field: &'static str,
    parse: impl Fn(&NbtCompound) -> Result<T, ProviderParseError>,
    from_number: impl Fn(f64) -> T + Copy,
    default: T,
) -> Result<ProviderRef<T>, ProviderParseError> {
    if nbt.get(field).is_none() {
        return Ok(ProviderRef::inline(default));
    }
    provider_ref(nbt, field, parse, from_number)
}

fn refs<T>(
    nbt: &NbtCompound,
    field: &'static str,
    parse: impl Fn(&NbtCompound) -> Result<T, ProviderParseError>,
    from_number: impl Fn(f64) -> T + Copy,
) -> Result<Vec<ProviderRef<T>>, ProviderParseError> {
    let list = tag(nbt, field)?
        .list()
        .ok_or(ProviderParseError::InvalidField(field))?;
    let mut providers = Vec::new();
    for element in nbt_list_values(list) {
        match element {
            NbtTag::String(key) => {
                providers.push(ProviderRef::Reference(identifier(&key.to_string())?));
            }
            NbtTag::Compound(compound) => {
                providers.push(ProviderRef::inline(parse(&compound)?));
            }
            NbtTag::Byte(value) => {
                providers.push(ProviderRef::inline(from_number(f64::from(value))));
            }
            NbtTag::Short(value) => {
                providers.push(ProviderRef::inline(from_number(f64::from(value))));
            }
            NbtTag::Int(value) => {
                providers.push(ProviderRef::inline(from_number(f64::from(value))));
            }
            NbtTag::Long(value) => providers.push(ProviderRef::inline(from_number(value as f64))),
            NbtTag::Float(value) => {
                providers.push(ProviderRef::inline(from_number(f64::from(value))));
            }
            NbtTag::Double(value) => providers.push(ProviderRef::inline(from_number(value))),
            _ => return Err(ProviderParseError::InvalidField(field)),
        }
    }
    Ok(providers)
}

fn float_ref(
    nbt: &NbtCompound,
    field: &'static str,
) -> Result<ProviderRef<FloatProvider>, ProviderParseError> {
    provider_ref(nbt, field, parse_float_provider, |value| {
        FloatProvider::Constant(value as f32)
    })
}

fn int_ref(
    nbt: &NbtCompound,
    field: &'static str,
) -> Result<ProviderRef<IntProvider>, ProviderParseError> {
    provider_ref(nbt, field, parse_int_provider, |value| {
        IntProvider::Constant(value as i32)
    })
}

fn optional_float_ref(
    nbt: &NbtCompound,
    field: &'static str,
) -> Result<ProviderRef<FloatProvider>, ProviderParseError> {
    // `ContextFloatProviders.exactly(0.0F)`.
    optional_ref(
        nbt,
        field,
        parse_float_provider,
        |value| FloatProvider::Constant(value as f32),
        FloatProvider::Constant(0.0),
    )
}

fn optional_int_ref(
    nbt: &NbtCompound,
    field: &'static str,
) -> Result<ProviderRef<IntProvider>, ProviderParseError> {
    // `ContextIntProviders.exactly(0)`.
    optional_ref(
        nbt,
        field,
        parse_int_provider,
        |value| IntProvider::Constant(value as i32),
        IntProvider::Constant(0),
    )
}

fn float_refs(
    nbt: &NbtCompound,
    field: &'static str,
) -> Result<Vec<ProviderRef<FloatProvider>>, ProviderParseError> {
    refs(nbt, field, parse_float_provider, |value| {
        FloatProvider::Constant(value as f32)
    })
}

fn int_refs(
    nbt: &NbtCompound,
    field: &'static str,
) -> Result<Vec<ProviderRef<IntProvider>>, ProviderParseError> {
    refs(nbt, field, parse_int_provider, |value| {
        IntProvider::Constant(value as i32)
    })
}

/// The parsed `cases` list plus the `default` slot of a dispatcher provider.
type DispatcherCases<T> = (Vec<(ConditionRef, ProviderRef<T>)>, ProviderRef<T>);

fn dispatcher<T>(
    nbt: &NbtCompound,
    parse: impl Fn(&NbtCompound) -> Result<T, ProviderParseError> + Copy,
    from_number: impl Fn(f64) -> T + Copy,
    default_zero: impl FnOnce() -> T,
) -> Result<DispatcherCases<T>, ProviderParseError> {
    let cases = tag(nbt, "cases")?
        .list()
        .ok_or(ProviderParseError::InvalidField("cases"))?;
    let mut parsed = Vec::new();
    for case in nbt_list_values(cases) {
        let NbtTag::Compound(case) = case else {
            return Err(ProviderParseError::InvalidField("cases"));
        };
        parsed.push((
            condition_ref(&case, "condition")?,
            provider_ref(&case, "value", parse, from_number)?,
        ));
    }
    let default = if nbt.get("default").is_some() {
        provider_ref(nbt, "default", parse, from_number)?
    } else {
        // The Java default is the family's `exactly(0)` holder.
        ProviderRef::inline(default_zero())
    };
    Ok((parsed, default))
}

fn weighted_list<T>(
    nbt: &NbtCompound,
    parse: impl Fn(&NbtCompound) -> Result<T, ProviderParseError> + Copy,
    from_number: impl Fn(f64) -> T + Copy,
) -> Result<WeightedList<T>, ProviderParseError> {
    let distribution = tag(nbt, "distribution")?
        .list()
        .ok_or(ProviderParseError::InvalidField("distribution"))?;
    let mut entries = Vec::new();
    for entry in nbt_list_values(distribution) {
        let NbtTag::Compound(entry) = entry else {
            return Err(ProviderParseError::InvalidField("distribution"));
        };
        let weight = entry
            .int("weight")
            .ok_or(ProviderParseError::MissingField("weight"))?;
        if weight < 0 {
            return Err(ProviderParseError::InvalidField("weight"));
        }
        entries.push(WeightedValue::new(
            provider_ref(&entry, "data", parse, from_number)?,
            weight,
        ));
    }
    Ok(WeightedList::new(entries))
}

fn storage_access(
    nbt: &NbtCompound,
) -> Result<(Identifier, vecta_utils::nbt::NbtPath), ProviderParseError> {
    let storage = nbt
        .string("storage")
        .ok_or(ProviderParseError::MissingField("storage"))?
        .to_string();
    let storage = identifier(&storage)?;
    let path = nbt
        .string("path")
        .ok_or(ProviderParseError::MissingField("path"))?
        .to_string();
    let path = parse_nbt_path(&path).map_err(|_| ProviderParseError::InvalidValue("path"))?;
    Ok((storage, path))
}

fn condition_ref(
    nbt: &NbtCompound,
    field: &'static str,
) -> Result<ConditionRef, ProviderParseError> {
    match tag(nbt, field)? {
        NbtTag::String(key) => Ok(ConditionRef::Reference(identifier(&key.to_string())?)),
        // An inline condition needs an owned representation; the compiled
        // `LootCondition` borrows `'static` property tables, so a runtime payload
        // would have to leak them. Vanilla configured values only reference
        // predicates, which stay supported.
        NbtTag::Compound(_) => Err(ProviderParseError::UnsupportedType(
            "inline loot condition".to_owned(),
        )),
        _ => Err(ProviderParseError::InvalidField(field)),
    }
}

fn identifier(value: &str) -> Result<Identifier, ProviderParseError> {
    Identifier::from_str(value).map_err(|_| ProviderParseError::InvalidValue("identifier"))
}
