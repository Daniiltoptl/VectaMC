//! Server-wide random sequences shared by all loaded worlds.
//!
//! The stream state and seed defaults match Vanilla's `RandomSequences`.
//! Persistence uses Vecta's typed saved-data TOML boundary rather than Vanilla NBT.

use std::{collections::BTreeMap, io, sync::Arc};

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use vecta_utils::{
    Identifier,
    locks::{AsyncMutex, SyncMutex},
    random::{RandomSource, random_sequence, xoroshiro::Xoroshiro},
    saved_data::{SavedDataManager, names::RANDOM_SEQUENCES},
};

use super::worlds::WorldMap;

#[derive(Deserialize, Serialize)]
#[serde(default)]
struct PersistentRandomSequences {
    salt: i32,
    include_world_seed: bool,
    include_sequence_id: bool,
    sequences: BTreeMap<String, PersistentSequence>,
}

impl Default for PersistentRandomSequences {
    fn default() -> Self {
        Self {
            salt: 0,
            include_world_seed: true,
            include_sequence_id: true,
            sequences: BTreeMap::new(),
        }
    }
}

#[derive(Deserialize, Serialize)]
struct PersistentSequence {
    source: [i64; 2],
}

struct SequenceState {
    salt: i32,
    include_world_seed: bool,
    include_sequence_id: bool,
    sequences: FxHashMap<Identifier, RandomSource>,
    revision: u64,
    saved_revision: u64,
}

struct SaveSnapshot {
    revision: u64,
    data: PersistentRandomSequences,
}

/// The server's named Xoroshiro streams and their durable state.
pub(crate) struct RandomSequences {
    world_seed: i64,
    state: SyncMutex<SequenceState>,
    save_lock: AsyncMutex<()>,
}

impl RandomSequences {
    pub(crate) fn new(world_seed: i64) -> Self {
        Self {
            world_seed,
            state: SyncMutex::new(SequenceState {
                salt: 0,
                include_world_seed: true,
                include_sequence_id: true,
                sequences: FxHashMap::default(),
                revision: 0,
                saved_revision: 0,
            }),
            save_lock: AsyncMutex::new(()),
        }
    }

    pub(crate) async fn load_and_bind(worlds: &WorldMap) -> io::Result<Arc<Self>> {
        let primary = worlds.server_default_world().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "server random sequences need a default world",
            )
        })?;
        let world_seed = primary.level_data.read().seed();
        let sequences = Arc::new(Self::load(world_seed, &primary.saved_data).await?);
        for world in worlds.values() {
            world.random().bind_sequences(Arc::clone(&sequences));
        }
        Ok(sequences)
    }

    async fn load(world_seed: i64, storage: &SavedDataManager) -> io::Result<Self> {
        let data: PersistentRandomSequences = storage.load_or_default(RANDOM_SEQUENCES).await?;
        let mut sequences = FxHashMap::default();
        for (key, sequence) in data.sequences {
            let key = key
                .parse::<Identifier>()
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            let [lo, hi] = sequence.source;
            sequences.insert(
                key,
                RandomSource::Xoroshiro(Xoroshiro::from_seed_128(lo as u64, hi as u64)),
            );
        }
        Ok(Self {
            world_seed,
            state: SyncMutex::new(SequenceState {
                salt: data.salt,
                include_world_seed: data.include_world_seed,
                include_sequence_id: data.include_sequence_id,
                sequences,
                revision: 0,
                saved_revision: 0,
            }),
            save_lock: AsyncMutex::new(()),
        })
    }

    pub(crate) fn with_sequence<T>(
        &self,
        key: &Identifier,
        f: impl FnOnce(&mut RandomSource) -> T,
    ) -> T {
        let mut state = self.state.lock();
        let seed = if state.include_world_seed {
            self.world_seed
        } else {
            0
        } ^ i64::from(state.salt);
        let include_sequence_id = state.include_sequence_id;
        state.revision = state.revision.wrapping_add(1);
        let source = state.sequences.entry(key.clone()).or_insert_with(|| {
            if include_sequence_id {
                random_sequence(seed, &key.to_string())
            } else {
                RandomSource::Xoroshiro(Xoroshiro::from_seed(seed as u64))
            }
        });
        f(source)
    }

    fn pending_save(&self) -> io::Result<Option<SaveSnapshot>> {
        let state = self.state.lock();
        if state.revision == state.saved_revision {
            return Ok(None);
        }
        let mut sequences = BTreeMap::new();
        for (key, source) in &state.sequences {
            let RandomSource::Xoroshiro(source) = source else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "a named random sequence must remain Xoroshiro",
                ));
            };
            sequences.insert(
                key.to_string(),
                PersistentSequence {
                    source: source.seed_words(),
                },
            );
        }
        Ok(Some(SaveSnapshot {
            revision: state.revision,
            data: PersistentRandomSequences {
                salt: state.salt,
                include_world_seed: state.include_world_seed,
                include_sequence_id: state.include_sequence_id,
                sequences,
            },
        }))
    }

    pub(crate) async fn save(&self, storage: &SavedDataManager) -> io::Result<bool> {
        let _save_guard = self.save_lock.lock().await;
        let Some(snapshot) = self.pending_save()? else {
            return Ok(false);
        };
        storage.save(RANDOM_SEQUENCES, &snapshot.data).await?;
        self.mark_saved(snapshot.revision);
        Ok(true)
    }

    fn mark_saved(&self, revision: u64) {
        self.state.lock().saved_revision = revision;
    }
}

#[cfg(test)]
mod tests;
