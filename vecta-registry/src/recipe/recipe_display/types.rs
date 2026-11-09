use std::fmt::{self, Debug, Formatter};
use std::io::{Cursor, Result, Write};

use rustc_hash::FxHashMap;
use vecta_utils::Identifier;

use super::{RecipeDisplay, RecipeDisplayKind, RecipeDisplayTypeRef};

/// Registered recipe display type. Its numeric ID is assigned by the
/// registry, independently of the Rust payload kind.
pub struct RecipeDisplayType {
    pub key: Identifier,
    kind: RecipeDisplayKind,
}

impl RecipeDisplayType {
    pub(crate) const fn vanilla(key: Identifier, kind: RecipeDisplayKind) -> Self {
        Self { key, kind }
    }

    pub(super) const fn kind(&self) -> RecipeDisplayKind {
        self.kind
    }

    pub(super) fn read_payload(&self, data: &mut Cursor<&[u8]>) -> Result<RecipeDisplay> {
        RecipeDisplay::read_vanilla_payload(self.kind(), data)
    }

    pub(super) fn write_payload(
        &self,
        display: &RecipeDisplay,
        writer: &mut impl Write,
    ) -> Result<()> {
        if display.kind() == self.kind() {
            display.write_vanilla_payload(writer)
        } else {
            Err(std::io::Error::other(
                "Recipe display payload does not match its registered kind",
            ))
        }
    }
}

impl Debug for RecipeDisplayType {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RecipeDisplayType")
            .field("key", &self.key)
            .field("kind", &self.kind)
            .finish()
    }
}

/// Registry for protocol-visible recipe display types.
pub struct RecipeDisplayTypeRegistry {
    recipe_displays_by_id: Vec<RecipeDisplayTypeRef>,
    recipe_displays_by_key: FxHashMap<Identifier, usize>,
    allows_registering: bool,
}

impl RecipeDisplayTypeRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            recipe_displays_by_id: Vec::new(),
            recipe_displays_by_key: FxHashMap::default(),
            allows_registering: true,
        }
    }

    pub(super) fn registered_entry_with_id(
        &self,
        entry: RecipeDisplayTypeRef,
    ) -> Option<(usize, RecipeDisplayTypeRef)> {
        let id = self.recipe_displays_by_key.get(&entry.key).copied()?;
        let registered = self.recipe_displays_by_id.get(id).copied()?;
        std::ptr::eq(registered, entry).then_some((id, registered))
    }
}

crate::impl_standard_methods!(
    RecipeDisplayTypeRegistry,
    RecipeDisplayTypeRef,
    recipe_displays_by_id,
    recipe_displays_by_key,
    allows_registering,
    "Cannot register duplicate recipe display key: {}"
);
crate::impl_registry!(
    RecipeDisplayTypeRegistry,
    RecipeDisplayType,
    recipe_displays_by_id,
    recipe_displays_by_key,
    recipe_displays
);
