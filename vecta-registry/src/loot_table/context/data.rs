//! Read-only level services used by context-dependent loot number providers.

use uuid::Uuid;
use vecta_utils::{Identifier, nbt::NbtPath};

/// Players use their profile name; other vanilla entities use their UUID.
#[derive(Debug, Clone, Copy)]
pub enum LootScoreHolder<'a> {
    Name(&'a str),
    Entity(Uuid),
}

/// Reads current domain data without exposing server ownership or write access.
pub trait LootDataReader: Send + Sync {
    fn score(&self, holder: LootScoreHolder<'_>, objective: &str) -> Option<i32>;
    /// Returns a number only when the path selects exactly one numeric tag.
    fn storage_float(&self, storage: &Identifier, path: &NbtPath) -> Option<f32>;
    /// Uses the underlying boxed-number conversion, including long narrowing.
    fn storage_int(&self, storage: &Identifier, path: &NbtPath) -> Option<i32>;
}
