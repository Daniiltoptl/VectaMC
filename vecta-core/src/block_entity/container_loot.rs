//! Loot table state shared by Vanilla `RandomizableContainer` block entities.
//!
//! A container that carries a loot table keeps it — and its seed — in the block
//! entity until the first consumer reads the contents. Only then is the table
//! rolled and dropped, so opening, a comparator read or a chunk save can never
//! generate the same loot twice.

use std::str::FromStr as _;

use glam::DVec3;
use simdnbt::borrow::BaseNbtCompound as BorrowedNbtCompound;
use simdnbt::borrow::NbtCompound as NbtCompoundView;
use simdnbt::owned::NbtCompound;
use vecta_registry::data_components::DataComponentMap;
use vecta_registry::data_components::components::SeededContainerLoot;
use vecta_registry::data_components::vanilla_components::CONTAINER_LOOT;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::loot_table::LootContext;
use vecta_registry::{REGISTRY, RegistryExt as _};
use vecta_utils::{BlockPos, Identifier, locks::SyncMutex};

use crate::entity::{LivingEntity as _, entity_loot_ref, loot_equipment};
use crate::player::Player;

/// NBT key holding the loot table resource location.
pub const LOOT_TABLE_TAG: &str = "LootTable";
/// NBT key holding the loot table seed.
pub const LOOT_TABLE_SEED_TAG: &str = "LootTableSeed";

/// Pending loot table of one container block entity.
#[derive(Default)]
pub struct ContainerLoot {
    state: SyncMutex<ContainerLootState>,
}

#[derive(Default)]
struct ContainerLootState {
    table: Option<Identifier>,
    seed: i64,
}

impl ContainerLoot {
    /// Creates a container with no pending loot table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Vanilla `RandomizableContainer.tryLoadLootTable`.
    ///
    /// Returns whether a loot table was stored, in which case the container's
    /// `Items` must not be read: the table replaces them.
    pub fn load(&self, nbt: &BorrowedNbtCompound<'_>) -> bool {
        let nbt: NbtCompoundView<'_, '_> = nbt.into();
        let table = nbt
            .string(LOOT_TABLE_TAG)
            .and_then(|value| Identifier::from_str(&value.to_string()).ok());
        let seed = nbt.long(LOOT_TABLE_SEED_TAG).unwrap_or(0);
        let mut state = self.state.lock();
        state.table.clone_from(&table);
        state.seed = seed;
        table.is_some()
    }

    /// Vanilla `RandomizableContainer.trySaveLootTable`.
    ///
    /// Returns whether a loot table is still pending, in which case the
    /// container must not write its `Items`.
    pub fn save(&self, nbt: &mut NbtCompound) -> bool {
        let state = self.state.lock();
        let Some(table) = &state.table else {
            return false;
        };
        nbt.insert(LOOT_TABLE_TAG, table.to_string());
        if state.seed != 0 {
            nbt.insert(LOOT_TABLE_SEED_TAG, state.seed);
        }
        true
    }

    /// Returns whether the container still has to roll its loot.
    #[must_use]
    pub fn is_randomized(&self) -> bool {
        self.state.lock().table.is_some()
    }

    /// Sets the pending loot table and its seed.
    pub fn set(&self, table: Option<Identifier>, seed: i64) {
        let mut state = self.state.lock();
        state.table = table;
        state.seed = seed;
    }

    /// Vanilla `RandomizableContainer.applyImplicitComponents`: an item's
    /// `minecraft:container_loot` component wins over the stored table.
    pub fn apply_implicit_components(&self, item: &ItemStack) -> Option<Identifier> {
        let loot = item.get(CONTAINER_LOOT)?;
        let mut state = self.state.lock();
        state.table = Some(loot.loot_table().clone());
        state.seed = loot.seed();
        Some(CONTAINER_LOOT.key().clone())
    }

    /// Vanilla `RandomizableContainer.collectImplicitComponents`.
    pub fn collect_implicit_components(&self, components: &mut DataComponentMap) {
        let state = self.state.lock();
        if let Some(table) = &state.table {
            components.set(
                CONTAINER_LOOT,
                Some(SeededContainerLoot::new(table.clone(), state.seed)),
            );
        }
    }

    /// Vanilla `RandomizableContainer.removeComponentsFromTag`.
    pub fn remove_components_from_tag(nbt: &mut NbtCompound) {
        while nbt.remove(LOOT_TABLE_TAG).is_some() {}
        while nbt.remove(LOOT_TABLE_SEED_TAG).is_some() {}
    }

    /// Vanilla `RandomizableContainer.unpackLootTable`: rolls the pending table
    /// into `slots` once and forgets it.
    ///
    /// The table reference is cleared before the roll, so a failure to resolve
    /// the table cannot leave a container that would roll again on the next
    /// read.
    pub fn unpack(&self, pos: BlockPos, slots: &mut [ItemStack], player: Option<&Player>) {
        let key = {
            let mut state = self.state.lock();
            let Some(key) = state.table.take() else {
                return;
            };
            state.seed = 0;
            key
        };
        let Some(table) = REGISTRY.loot_tables.by_key(&key) else {
            log::warn!("loot table {key} is not registered; the container stays empty");
            return;
        };

        let center = DVec3::new(
            f64::from(pos.x()) + 0.5,
            f64::from(pos.y()) + 0.5,
            f64::from(pos.z()) + 0.5,
        );
        let mut rng = rand::rng();
        let mut ctx = LootContext::new(&mut rng).with_origin(center.x, center.y, center.z);
        let equipment = player.and_then(|player| loot_equipment(player));
        if let Some(player) = player {
            ctx = ctx
                .with_luck(player.get_luck())
                .with_interacting_entity(entity_loot_ref(player, equipment.as_ref()));
        }

        table.fill(slots, &mut ctx);
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use simdnbt::borrow::read_compound as read_borrowed_compound;
    use vecta_registry::init_vanilla_registry;

    use super::*;

    fn round_trip(loot: &ContainerLoot) -> ContainerLoot {
        let mut nbt = NbtCompound::new();
        assert!(loot.save(&mut nbt));
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("loot NBT reads");
        let loaded = ContainerLoot::new();
        assert!(loaded.load(&borrowed));
        loaded
    }

    #[test]
    fn a_pending_table_survives_a_disk_round_trip() {
        init_vanilla_registry();
        let loot = ContainerLoot::new();
        loot.set(
            Some(Identifier::vanilla_static("chests/simple_dungeon")),
            42,
        );

        let loaded = round_trip(&loot);

        assert!(loaded.is_randomized());
        let state = loaded.state.lock();
        assert_eq!(
            state.table.as_ref().map(ToString::to_string),
            Some("minecraft:chests/simple_dungeon".to_owned()),
        );
        assert_eq!(state.seed, 42);
    }

    #[test]
    fn a_container_without_loot_writes_no_tags() {
        init_vanilla_registry();
        let loot = ContainerLoot::new();

        let mut nbt = NbtCompound::new();

        assert!(!loot.save(&mut nbt));
        assert!(nbt.get(LOOT_TABLE_TAG).is_none());
        assert!(nbt.get(LOOT_TABLE_SEED_TAG).is_none());
    }

    #[test]
    fn unpacking_rolls_the_table_once_and_clears_it() {
        init_vanilla_registry();
        let loot = ContainerLoot::new();
        loot.set(Some(Identifier::vanilla_static("chests/simple_dungeon")), 0);
        let mut slots = vec![ItemStack::empty(); 27];

        loot.unpack(BlockPos::new(0, 64, 0), &mut slots, None);

        assert!(!loot.is_randomized(), "the table is consumed exactly once");
        let mut nbt = NbtCompound::new();
        assert!(!loot.save(&mut nbt), "a consumed table is not saved again");
        assert!(
            slots.iter().any(|stack| !stack.is_empty()),
            "a dungeon chest rolls at least one item",
        );
    }

    #[test]
    fn a_consumed_table_does_not_roll_again() {
        init_vanilla_registry();
        let loot = ContainerLoot::new();
        loot.set(Some(Identifier::vanilla_static("chests/simple_dungeon")), 0);
        let mut slots = vec![ItemStack::empty(); 27];
        loot.unpack(BlockPos::new(0, 64, 0), &mut slots, None);
        let first: Vec<ItemStack> = slots.clone();

        loot.unpack(BlockPos::new(0, 64, 0), &mut slots, None);

        assert_eq!(slots.len(), first.len());
        for (before, after) in first.iter().zip(&slots) {
            assert_eq!(before.is_empty(), after.is_empty());
        }
    }
}
