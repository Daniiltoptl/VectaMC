//! Shared domain state for read-only loot queries; it owns no world or server.

use std::sync::Arc;

use simdnbt::owned::NbtTag;
use vecta_registry::loot_table::{LootDataReader, LootScoreHolder};
use vecta_utils::{
    Identifier,
    nbt::{NbtNumeric as _, NbtPath},
};

use super::World;
use crate::command::storage::{CommandStorage, DomainCommandStorage};
use crate::scoreboard::{DomainScoreboards, ScoreHolder, Scoreboard};
use crate::server::worlds::WorldMap;

pub(crate) struct DomainLootData {
    scoreboard: Arc<Scoreboard>,
    storage: Arc<CommandStorage>,
}

impl DomainLootData {
    pub(crate) fn bind(
        worlds: &WorldMap,
        scoreboards: &DomainScoreboards,
        storages: &DomainCommandStorage,
    ) -> Result<(), String> {
        for domain in worlds.domain_names() {
            let scoreboard = scoreboards
                .shared(domain)
                .ok_or_else(|| format!("Missing loot scoreboard for domain '{domain}'"))?;
            let storage = storages
                .shared(domain)
                .ok_or_else(|| format!("Missing loot storage for domain '{domain}'"))?;
            let data = Arc::new(Self {
                scoreboard,
                storage,
            });
            for world in worlds.values().filter(|world| world.domain() == domain) {
                *world.loot_data.write() = Some(Arc::clone(&data));
            }
        }
        Ok(())
    }

    fn storage_number(&self, storage: &Identifier, path: &NbtPath) -> Option<NbtTag> {
        // Each query reads current state. These temporary NBT values are not a
        // second domain storage or a cached snapshot of it.
        let root = NbtTag::Compound(self.storage.get(storage));
        let mut selected = path.get(&root).into_iter();
        let tag = selected.next()?;
        if selected.next().is_some() {
            return None;
        }
        tag.codec_f64()?;
        Some(tag)
    }
}

impl World {
    fn domain_loot_data(&self) -> Option<Arc<DomainLootData>> {
        self.loot_data.read().as_ref().map(Arc::clone)
    }
}

impl LootDataReader for World {
    fn score(&self, holder: LootScoreHolder<'_>, objective: &str) -> Option<i32> {
        let data = self.domain_loot_data()?;
        let objective = data.scoreboard.objective(objective)?;
        let name = match holder {
            LootScoreHolder::Name(name) => name.to_owned(),
            LootScoreHolder::Entity(uuid) => uuid.to_string(),
        };
        data.scoreboard.score(&ScoreHolder::new(name), &objective)
    }

    fn storage_float(&self, storage: &Identifier, path: &NbtPath) -> Option<f32> {
        self.domain_loot_data()?
            .storage_number(storage, path)
            .and_then(|tag| tag.codec_f32())
    }

    fn storage_int(&self, storage: &Identifier, path: &NbtPath) -> Option<i32> {
        self.domain_loot_data()?
            .storage_number(storage, path)
            .and_then(|tag| tag.codec_i32())
    }
}
