//! Chest block entity implementation.
//!
//! A chest stores 27 slots and tracks how many players have its menu open so
//! clients can animate the lid through block events.

use std::{
    mem,
    sync::{Arc, Weak},
};

use glam::DVec3;

use simdnbt::ToNbtTag;
use simdnbt::borrow::{BaseNbtCompound as BorrowedNbtCompound, NbtCompound as NbtCompoundView};
use simdnbt::owned::{NbtCompound, NbtList, NbtTag};
use vecta_protocol::packets::game::SoundSource;
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::blocks::properties::{BlockStateProperties, ChestType};
use vecta_registry::data_components::DataComponentMap;
use vecta_registry::data_components::vanilla_components::CONTAINER;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::item_stack_template::ItemStackTemplate;
use vecta_registry::{sound_events, vanilla_block_entity_types};
use vecta_utils::{
    BlockPos, BlockStateId, DowncastType, DowncastTypeKey, Identifier, locks::SyncMutex,
};

use crate::block_entity::container_component::{
    apply_container_items, collect_container_component,
};
use crate::block_entity::container_identity::ContainerIdentity;
use crate::block_entity::container_loot::ContainerLoot;
use crate::block_entity::container_openers::{ContainerOpenersCounter, ContainerOpenersListener};
use crate::block_entity::{BlockEntity, BlockEntityBase, BlockEntityLifecycleExt as _};
use crate::entity::Entity as _;
use crate::inventory::container::Container;
use crate::inventory::lock::{ContainerRef, SharedContainer};
use crate::player::Player;
use crate::world::World;

/// Number of slots in a chest (3 rows of 9).
pub const CHEST_SLOTS: usize = 27;

/// Block event id clients use to drive the chest lid.
pub const CHEST_LID_EVENT: i32 = 1;

/// Chest block entity.
pub struct ChestBlockEntity {
    base: Arc<BlockEntityBase>,
    container: Arc<SyncMutex<ChestContainer>>,
    container_ref: ContainerRef,
    openers: ContainerOpenersCounter,
    identity: ContainerIdentity,
    loot: ContainerLoot,
}

struct ChestContainer {
    items: Vec<ItemStack>,
}

// SAFETY: This key is owned by Vecta and uniquely identifies `ChestBlockEntity`.
unsafe impl DowncastType for ChestBlockEntity {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:block_entity/chest");
}

// SAFETY: This key is owned by Vecta and uniquely identifies the independently
// lockable inventory data used by a chest block entity.
unsafe impl DowncastType for ChestContainer {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:container/chest");
}

impl ChestBlockEntity {
    /// Creates a new chest block entity.
    #[must_use]
    pub fn new(level: Weak<World>, pos: BlockPos, state: BlockStateId) -> Self {
        let base = Arc::new(BlockEntityBase::new(
            &vanilla_block_entity_types::CHEST,
            level,
            pos,
            state,
        ));
        let container = Arc::new(SyncMutex::new(ChestContainer {
            items: vec![ItemStack::empty(); CHEST_SLOTS],
        }));
        let shared_container: SharedContainer = Arc::<SyncMutex<ChestContainer>>::clone(&container);
        Self {
            container_ref: ContainerRef::owned_by_block_entity(shared_container, Arc::clone(&base)),
            base,
            container,
            openers: ContainerOpenersCounter::default(),
            identity: ContainerIdentity::new(),
            loot: ContainerLoot::new(),
        }
    }

    /// Returns the custom name and lock of this chest.
    #[must_use]
    pub const fn identity(&self) -> &ContainerIdentity {
        &self.identity
    }

    /// Rolls this chest's pending loot table, if any.
    ///
    /// Vanilla `RandomizableContainerBlockEntity` does this on every contents
    /// read, so opening, a comparator read and a destruction drop all see the
    /// rolled contents while only the first one generates them.
    pub fn unpack_loot_table(&self, player: Option<&Player>) {
        self.loot.unpack(
            self.get_block_pos(),
            &mut self.container.lock().items,
            player,
        );
    }

    /// Returns how many players currently have this chest open.
    #[must_use]
    pub fn open_count(&self) -> i32 {
        self.openers.opener_count()
    }

    /// Records a player opening this chest's menu (`ChestBlockEntity.startOpen`).
    pub fn start_open(&self, player: &Player) {
        if self.is_removed() || player.is_spectator() {
            return;
        }
        let Some(world) = self.get_level() else {
            return;
        };
        self.openers.increment_openers(
            self,
            player,
            &world,
            self.get_block_pos(),
            self.get_block_state(),
        );
    }

    /// Records a player closing this chest's menu (`ChestBlockEntity.stopOpen`).
    pub fn stop_open(&self, player: &Player) {
        if self.is_removed() || player.is_spectator() {
            return;
        }
        let Some(world) = self.get_level() else {
            return;
        };
        self.openers.decrement_openers(
            self,
            player,
            &world,
            self.get_block_pos(),
            self.get_block_state(),
        );
    }

    /// Plays a lid sound from the chest's center; for a double chest only the
    /// right half plays it, from the middle of both halves.
    fn play_lid_sound(world: &Arc<World>, pos: BlockPos, state: BlockStateId, opening: bool) {
        let chest_type = state
            .try_get_value(&BlockStateProperties::CHEST_TYPE)
            .unwrap_or(ChestType::Single);
        if chest_type == ChestType::Left {
            return;
        }
        let mut sound_pos = DVec3::new(
            f64::from(pos.x()) + 0.5,
            f64::from(pos.y()) + 0.5,
            f64::from(pos.z()) + 0.5,
        );
        if chest_type == ChestType::Right
            && let Some(facing) = state.try_get_value(&BlockStateProperties::HORIZONTAL_FACING)
        {
            // `ChestBlock.getConnectedDirection` for the right half.
            let (dx, _, dz) = facing.rotate_y_counter_clockwise().offset();
            sound_pos.x += f64::from(dx) * 0.5;
            sound_pos.z += f64::from(dz) * 0.5;
        }
        let sound = if opening {
            &sound_events::BLOCK_CHEST_OPEN
        } else {
            &sound_events::BLOCK_CHEST_CLOSE
        };
        world.play_sound_at(
            sound,
            SoundSource::Blocks,
            sound_pos,
            0.5,
            rand::random::<f32>() * 0.1 + 0.9,
            None,
        );
    }
}

impl ContainerOpenersListener for ChestBlockEntity {
    fn on_open(&self, world: &Arc<World>, pos: BlockPos, state: BlockStateId) {
        Self::play_lid_sound(world, pos, state, true);
    }

    fn on_close(&self, world: &Arc<World>, pos: BlockPos, state: BlockStateId) {
        Self::play_lid_sound(world, pos, state, false);
    }

    fn opener_count_changed(
        &self,
        world: &Arc<World>,
        pos: BlockPos,
        state: BlockStateId,
        _previous: i32,
        current: i32,
    ) {
        // `ChestBlockEntity.signalOpenCount`.
        world.block_event(pos, state.get_block(), CHEST_LID_EVENT, current);
    }
}

impl BlockEntity for ChestBlockEntity {
    fn base(&self) -> &BlockEntityBase {
        &self.base
    }

    fn pre_remove_side_effects(&self, pos: BlockPos, _state: BlockStateId) {
        // Vanilla `Containers.dropContents` reads the slots, which unpacks the
        // loot table first.
        self.unpack_loot_table(None);
        let items = {
            let mut container = self.container.lock();
            mem::replace(&mut container.items, vec![ItemStack::empty(); CHEST_SLOTS])
        };
        let Some(world) = self.get_level() else {
            return;
        };
        for item in items {
            world.drop_item_stack(pos, item);
        }
    }

    fn load_additional(&self, nbt: &BorrowedNbtCompound<'_>) {
        let nbt_view: NbtCompoundView<'_, '_> = nbt.into();
        self.identity.load(nbt);
        let mut container = self.container.lock();
        container.items.fill(ItemStack::empty());
        if self.loot.load(nbt) {
            // Vanilla `RandomizableContainerBlockEntity.loadFromTag`: a stored
            // loot table replaces the item list, it does not accompany it.
            return;
        }

        if let Some(items_list) = nbt_view.list("Items")
            && let Some(compounds) = items_list.compounds()
        {
            for compound in compounds {
                if let Some(slot) = compound.byte("Slot") {
                    let slot = slot as usize;
                    if slot < CHEST_SLOTS
                        && let Some(item) = ItemStack::from_borrowed_compound(&compound)
                    {
                        container.items[slot] = item;
                    }
                }
            }
        }
    }

    fn save_additional(&self, nbt: &mut NbtCompound) {
        self.identity.save(nbt);
        if self.loot.save(nbt) {
            // `trySaveLootTable` succeeded, so the item list stays out of the
            // save: the table regenerates it on the next read.
            return;
        }
        let container = self.container.lock();
        let mut items: Vec<NbtCompound> = Vec::new();
        for (slot, item) in container.items.iter().enumerate() {
            if !item.is_empty()
                && let NbtTag::Compound(mut item_nbt) = item.clone().to_nbt_tag()
            {
                item_nbt.insert("Slot", slot as i8);
                items.push(item_nbt);
            }
        }
        nbt.insert("Items", NbtList::Compound(items));
    }

    fn apply_implicit_components(&self, item: &ItemStack) -> Vec<Identifier> {
        let mut consumed = self.identity.apply_implicit_components(item);
        if let Some(key) = self.loot.apply_implicit_components(item) {
            consumed.push(key);
        }
        if let Some(contents) = item.get(CONTAINER) {
            // `ItemContainerContents.copyInto` keeps slot positions, so empty
            // entries stay empty instead of collapsing the list.
            let items = contents
                .items()
                .iter()
                .map(|entry| {
                    entry
                        .as_ref()
                        .map_or_else(ItemStack::empty, ItemStackTemplate::create)
                })
                .collect();
            apply_container_items(&mut self.container.lock().items, items);
            consumed.push(CONTAINER.key().clone());
        }
        consumed
    }

    fn collect_implicit_components(&self, components: &mut DataComponentMap) {
        self.identity.collect_implicit_components(components);
        self.loot.collect_implicit_components(components);
        let contents = collect_container_component(&self.container.lock().items);
        match contents {
            Ok(contents) => components.set(CONTAINER, Some(contents)),
            Err(error) => log::warn!(
                "failed to collect the container component of the chest at {:?}: {error}",
                self.get_block_pos()
            ),
        }
    }

    fn remove_components_from_tag(&self, nbt: &mut NbtCompound) {
        ContainerIdentity::remove_components_from_tag(nbt);
        ContainerLoot::remove_components_from_tag(nbt);
        while nbt.remove("Items").is_some() {}
    }

    fn trigger_event(&self, param_a: i32, _param_b: i32) -> bool {
        // The lid itself animates client-side; accepting the event lets the
        // server broadcast it.
        param_a == CHEST_LID_EVENT
    }

    fn container_ref(&self) -> Option<ContainerRef> {
        Some(self.container_ref.clone())
    }
}

impl Container for ChestContainer {
    fn items(&self) -> &[ItemStack] {
        &self.items
    }

    fn items_mut(&mut self) -> &mut [ItemStack] {
        &mut self.items
    }

    fn get_container_size(&self) -> usize {
        CHEST_SLOTS
    }

    fn set_item(&mut self, slot: usize, mut stack: ItemStack) {
        if slot < CHEST_SLOTS {
            let max_stack_size = self.get_max_stack_size_for_item(&stack);
            if !stack.is_empty() && stack.count() > max_stack_size {
                stack.set_count(max_stack_size);
            }
            self.items[slot] = stack;
        }
    }

    fn get_max_stack_size(&self) -> i32 {
        64
    }

    fn set_changed(&mut self) {}
}

#[cfg(test)]
mod tests {
    use vecta_registry::{init_vanilla_registry, vanilla_blocks, vanilla_items};

    use super::*;

    fn test_chest() -> ChestBlockEntity {
        init_vanilla_registry();
        ChestBlockEntity::new(
            Weak::new(),
            BlockPos::new(1, 2, 3),
            vanilla_blocks::CHEST.default_state(),
        )
    }

    #[test]
    fn pre_remove_clears_items() {
        let chest = test_chest();
        chest
            .container
            .lock()
            .set_item(0, ItemStack::new(&vanilla_items::STONE));

        chest.pre_remove_side_effects(
            BlockPos::new(1, 2, 3),
            vanilla_blocks::CHEST.default_state(),
        );

        let container = chest.container.lock();
        assert_eq!(container.items.len(), CHEST_SLOTS);
        assert!(container.items.iter().all(ItemStack::is_empty));
    }
}
