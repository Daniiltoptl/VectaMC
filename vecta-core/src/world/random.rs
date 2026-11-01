//! Vanilla `Level.random` and `RandomSequences`.
//!
//! Loot tables that carry no seed draw from one of two streams: the level's own
//! legacy source, or — when the table declares a `random_sequence` — a
//! Xoroshiro stream derived from the world seed and the table key. Vanilla
//! shares sequence states across the server; Vecta binds every hosted world
//! to its server's saved-data-backed sequence owner before game ticks start.

use std::sync::Arc;
use vecta_registry::loot_table::LootTable;
use vecta_utils::{
    Identifier,
    locks::{SyncMutex, SyncRwLock},
    random::{RandomSource, legacy_random::LegacyRandom},
};

use crate::server::random_sequences::RandomSequences;

use crate::world::World;

/// The random streams one loaded world hands out.
pub struct LevelRandom {
    /// Vanilla `Level.random`.
    level: SyncMutex<RandomSource>,
    /// Vanilla `RandomSequences`: one persisted stream per `random_sequence`.
    sequences: SyncRwLock<Arc<RandomSequences>>,
}

impl LevelRandom {
    /// Creates the streams of a world with the given seed.
    #[must_use]
    pub fn new(world_seed: i64) -> Self {
        Self {
            // Vanilla `Level.random = RandomSource.create()`: a legacy stream
            // seeded once with a unique seed, unrelated to the world seed.
            level: SyncMutex::new(RandomSource::Legacy(
                LegacyRandom::from_seed(rand::random()),
            )),
            // Standalone worlds own a stream bank until server initialization
            // binds them to the shared server bank, before gameplay starts.
            sequences: SyncRwLock::new(Arc::new(RandomSequences::new(world_seed))),
        }
    }

    /// Runs `f` against the level random source.
    pub fn with_level<T>(&self, f: impl FnOnce(&mut RandomSource) -> T) -> T {
        f(&mut self.level.lock())
    }

    /// Runs `f` against the stream of `key`, creating it on first use.
    pub fn with_sequence<T>(&self, key: &Identifier, f: impl FnOnce(&mut RandomSource) -> T) -> T {
        self.sequences.read().with_sequence(key, f)
    }

    pub(crate) fn bind_sequences(&self, sequences: Arc<RandomSequences>) {
        *self.sequences.write() = sequences;
    }

    /// Runs `f` against the stream Vanilla `LootContext.Builder.create` picks
    /// for this table: a nonzero loot seed wins, then the table's
    /// `random_sequence`, then the level random.
    pub fn with_loot<T>(
        &self,
        table: &LootTable,
        seed: i64,
        f: impl FnOnce(&mut RandomSource) -> T,
    ) -> T {
        if seed != 0 {
            let mut source = RandomSource::Legacy(LegacyRandom::from_seed(seed as u64));
            return f(&mut source);
        }
        match &table.random_sequence {
            Some(key) => self.with_sequence(key, f),
            None => self.with_level(f),
        }
    }
}

/// Runs `f` against the stream Vanilla would pick for `table`, falling back to a
/// fresh legacy stream when the caller has no level (a detached block entity).
pub(crate) fn with_loot_source<T>(
    world: Option<&World>,
    table: &LootTable,
    seed: i64,
    f: impl FnOnce(&mut RandomSource) -> T,
) -> T {
    if let Some(world) = world {
        world.random().with_loot(table, seed, f)
    } else {
        let mut source = RandomSource::Legacy(LegacyRandom::from_seed(rand::random()));
        f(&mut source)
    }
}

#[cfg(test)]
mod tests {
    use vecta_registry::loot_table::{LootTable, LootType};
    use vecta_utils::random::Random;

    use super::*;

    /// A table with no pools: enough to observe which stream `with_loot` picks.
    fn empty_table(random_sequence: Option<Identifier>) -> LootTable {
        LootTable {
            key: Identifier::vanilla_static("test/empty"),
            loot_type: LootType::Chest,
            pools: &[],
            functions: &[],
            random_sequence,
        }
    }

    #[test]
    fn a_loot_seed_outranks_both_level_streams() {
        let table = empty_table(Some(Identifier::vanilla_static("chests/simple_dungeon")));
        let roll = |seed: i64| {
            LevelRandom::new(9_999).with_loot(&table, seed, |rng| {
                (rng.next_i32(), rng.next_i32(), rng.next_f32())
            })
        };

        assert_eq!(roll(77), roll(77), "the same seed repeats exactly");
        assert_ne!(roll(77), roll(78), "a different seed rolls differently");
    }

    #[test]
    fn a_random_sequence_stream_is_shared_and_advances() {
        let table = empty_table(Some(Identifier::vanilla_static("chests/simple_dungeon")));
        let first_world = LevelRandom::new(1_234);
        let second_world = LevelRandom::new(1_234);

        let first = first_world.with_loot(&table, 0, Random::next_i32);
        let repeated = second_world.with_loot(&table, 0, Random::next_i32);
        assert_eq!(
            first, repeated,
            "the same world seed derives the same sequence state"
        );

        let advanced = first_world.with_loot(&table, 0, Random::next_i32);
        assert_ne!(
            first, advanced,
            "the sequence is persisted between rolls, not re-derived each time"
        );
    }

    #[test]
    fn a_table_without_a_sequence_draws_from_the_level_stream() {
        let table = empty_table(None);
        let world = LevelRandom::new(42);

        let first = world.with_loot(&table, 0, Random::next_i32);
        let second = world.with_loot(&table, 0, Random::next_i32);

        assert_ne!(first, second, "the level stream keeps advancing");
    }
}
