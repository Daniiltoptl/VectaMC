//! Registered parameter sets, populated by the build generator.
use super::ContextKeyId;
use rustc_hash::FxHashMap;
use vecta_utils::Identifier;

/// Registered parameter-set data is generated from `SteelExtractor` output.
pub struct ContextKeySet {
    pub key: Identifier,
    required: &'static [ContextKeyId],
    allowed: &'static [ContextKeyId],
}

impl ContextKeySet {
    #[must_use]
    pub const fn new(
        key: Identifier,
        required: &'static [ContextKeyId],
        allowed: &'static [ContextKeyId],
    ) -> Self {
        Self {
            key,
            required,
            allowed,
        }
    }

    #[must_use]
    pub const fn required(&self) -> &'static [ContextKeyId] {
        self.required
    }

    #[must_use]
    pub const fn allowed(&self) -> &'static [ContextKeyId] {
        self.allowed
    }
}

pub type ContextKeySetRef = &'static ContextKeySet;

pub struct ContextKeySetRegistry {
    sets_by_id: Vec<ContextKeySetRef>,
    sets_by_key: FxHashMap<Identifier, usize>,
    allows_registering: bool,
}

impl ContextKeySetRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sets_by_id: Vec::new(),
            sets_by_key: FxHashMap::default(),
            allows_registering: true,
        }
    }
}

crate::impl_standard_methods!(
    ContextKeySetRegistry,
    ContextKeySetRef,
    sets_by_id,
    sets_by_key,
    allows_registering,
    "Cannot register duplicate context key set: {}"
);
crate::impl_registry!(
    ContextKeySetRegistry,
    ContextKeySet,
    sets_by_id,
    sets_by_key,
    context_key_sets
);
