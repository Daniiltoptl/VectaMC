//! Parameter storage and validation, separate from key/value families and registry metadata.
use super::{ContextKey, ContextKeyId, ContextKeySet, ContextValue, ContextValueType};
use rustc_hash::FxHashMap;
use std::fmt::{self, Display, Formatter};

#[cfg(test)]
mod tests;

/// Runtime parameters only. Level state, luck and the mutable RNG remain on
/// `LootContext` rather than masquerading as vanilla loot parameters.
#[derive(Default)]
pub struct ContextMap<'a> {
    values: FxHashMap<ContextKeyId, ContextValue<'a>>,
}

impl<'a> ContextMap<'a> {
    pub fn set<T: ContextValueType>(&mut self, key: ContextKey<T>, value: Option<T::Value<'a>>) {
        if let Some(value) = value {
            self.values.insert(key.id(), T::encode(value));
        } else {
            self.values.remove(&key.id());
        }
    }

    #[must_use]
    pub fn get<T: ContextValueType>(&self, key: ContextKey<T>) -> Option<T::Value<'a>> {
        self.values.get(&key.id()).copied().and_then(T::decode)
    }

    pub fn get_required<T: ContextValueType>(
        &self,
        key: ContextKey<T>,
    ) -> Result<T::Value<'a>, ContextMapError> {
        self.get(key)
            .ok_or_else(|| ContextMapError::Missing(vec![key.id()]))
    }

    #[must_use]
    pub fn has(&self, key: ContextKeyId) -> bool {
        self.values.contains_key(&key)
    }

    /// Validate forbidden parameters before missing required ones, like Java.
    pub fn validate(&self, set: &ContextKeySet) -> Result<(), ContextMapError> {
        let mut forbidden: Vec<_> = self
            .values
            .keys()
            .filter(|key| !set.allowed().contains(key))
            .copied()
            .collect();
        if !forbidden.is_empty() {
            forbidden.sort_by_key(|key| key.name());
            return Err(ContextMapError::NotAllowed(forbidden));
        }
        let missing: Vec<_> = set
            .required()
            .iter()
            .filter(|key| !self.has(**key))
            .copied()
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(ContextMapError::Missing(missing))
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ContextMapError {
    NotAllowed(Vec<ContextKeyId>),
    Missing(Vec<ContextKeyId>),
}

impl Display for ContextMapError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let (message, keys) = match self {
            Self::NotAllowed(keys) => ("Parameters not allowed in this parameter set", keys),
            Self::Missing(keys) => ("Missing required parameters", keys),
        };
        write!(formatter, "{message}: [")?;
        for (index, key) in keys.iter().enumerate() {
            if index != 0 {
                formatter.write_str(", ")?;
            }
            write!(formatter, "<parameter {}>", key.name())?;
        }
        formatter.write_str("]")
    }
}
impl std::error::Error for ContextMapError {}
