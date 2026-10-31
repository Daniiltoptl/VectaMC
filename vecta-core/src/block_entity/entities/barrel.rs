//! Barrel block entity implementation.
//!
//! Barrels are container block entities with 27 slots (3x9 grid),
//! functioning similarly to chests but without double-chest behavior.

use std::{
    mem,
    sync::{Arc, Weak},
};

use simdnbt::ToNbtTag;
use simdnbt::borrow::{BaseNbtCompound as BorrowedNbtCompound, NbtCompound as NbtCompoundView};
use simdnbt::owned::{NbtCompound, NbtList, NbtTag};
use vecta_protocol::packets::game::SoundSource;
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::blocks::properties::BlockStateProperties;
use vecta_registry::data_components::DataComponentMap;
use vecta_registry::data_components::vanilla_components::CONTAINER;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::item_stack_template::ItemStackTemplate;
use vecta_registry::{sound_events, vanilla_block_entity_types};
use vecta_utils::types::UpdateFlags;
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

/// Number of slots in a barrel (3 rows of 9).
pub const BARREL_SLOTS: usize = 27;

/// Barrel block entity.
///
/// A simple container with 27 slots, using the same menu as chests.
pub struct BarrelBlockEntity {
    base: Arc<BlockEntityBase>,
    container: Arc<SyncMutex<BarrelContainer>>,
    container_ref: ContainerRef,
    identity: ContainerIdentity,
    loot: ContainerLoot,
    openers: ContainerOpenersCounter,
}

struct BarrelContainer {
    items: Vec<ItemStack>,
}

// SAFETY: This key is owned by Vecta and uniquely identifies `BarrelBlockEntity`.
unsafe impl DowncastType for BarrelBlockEntity {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:block_entity/barrel");
}

// SAFETY: This key is owned by Vecta and uniquely identifies the independently
// lockable inventory data used by a barrel block entity.
unsafe impl DowncastType for BarrelContainer {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:container/barrel");
}

impl BarrelBlockEntity {
    /// Creates a new barrel block entity.
    #[must_use]
    pub fn new(level: Weak<World>, pos: BlockPos, state: BlockStateId) -> Self {
        let base = Arc::new(BlockEntityBase::new(
            &vanilla_block_entity_types::BARREL,
            level,
            pos,
            state,
        ));
        let container = Arc::new(SyncMutex::new(BarrelContainer {
            items: vec![ItemStack::empty(); BARREL_SLOTS],
        }));
        let shared_container: SharedContainer =
            Arc::<SyncMutex<BarrelContainer>>::clone(&container);
        Self {
            container_ref: ContainerRef::owned_by_block_entity(shared_container, Arc::clone(&base)),
            base,
            container,
            identity: ContainerIdentity::new(),
            loot: ContainerLoot::new(),
            openers: ContainerOpenersCounter::default(),
        }
    }

    /// Returns the custom name and lock of this barrel.
    #[must_use]
    pub const fn identity(&self) -> &ContainerIdentity {
        &self.identity
    }

    /// Returns how many players currently have this barrel open.
    #[must_use]
    pub fn open_count(&self) -> i32 {
        self.openers.opener_count()
    }

    /// Records a player opening this barrel's menu (`BarrelBlockEntity.startOpen`).
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
            player.block_interaction_range(),
        );
    }

    /// Records a player closing this barrel's menu (`BarrelBlockEntity.stopOpen`).
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

    /// Vanilla `BarrelBlockEntity.recheckOpen`, driven by the block tick that
    /// `ContainerOpenersCounter` schedules while the barrel is open.
    pub fn recheck_open(&self) {
        if self.is_removed() {
            return;
        }
        let Some(world) = self.get_level() else {
            return;
        };
        self.openers
            .recheck_openers(self, &world, self.get_block_pos(), self.get_block_state());
    }

    /// Vanilla `BarrelBlockEntity.playSound`: the sound comes from the face the
    /// barrel opens towards.
    fn play_sound(world: &Arc<World>, pos: BlockPos, state: BlockStateId, opening: bool) {
        let facing = state.get_value(&BlockStateProperties::FACING);
        let (dx, dy, dz) = facing.offset();
        let sound = if opening {
            &sound_events::BLOCK_BARREL_OPEN
        } else {
            &sound_events::BLOCK_BARREL_CLOSE
        };
        world.play_sound_at(
            sound,
            SoundSource::Blocks,
            glam::DVec3::new(
                f64::from(pos.x()) + 0.5 + f64::from(dx) / 2.0,
                f64::from(pos.y()) + 0.5 + f64::from(dy) / 2.0,
                f64::from(pos.z()) + 0.5 + f64::from(dz) / 2.0,
            ),
            0.5,
            rand::random::<f32>() * 0.1 + 0.9,
            None,
        );
    }

    /// Vanilla `BarrelBlockEntity.updateBlockState`.
    fn update_open_state(world: &Arc<World>, pos: BlockPos, state: BlockStateId, is_open: bool) {
        world.set_block(
            pos,
            state.set_value(&BlockStateProperties::OPEN, is_open),
            UpdateFlags::UPDATE_ALL,
        );
    }

    /// Rolls this barrel's pending loot table, if any.
    pub fn unpack_loot_table(&self, player: Option<&Player>) {
        self.loot.unpack(
            self.get_block_pos(),
            &mut self.container.lock().items,
            player,
        );
    }
}

impl BlockEntity for BarrelBlockEntity {
    fn base(&self) -> &BlockEntityBase {
        &self.base
    }

    fn pre_remove_side_effects(&self, pos: BlockPos, _state: BlockStateId) {
        // Vanilla `Containers.dropContents` reads the slots, which unpacks the
        // loot table first.
        self.unpack_loot_table(None);
        let items = {
            let mut container = self.container.lock();
            mem::replace(&mut container.items, vec![ItemStack::empty(); BARREL_SLOTS])
        };
        let Some(world) = self.get_level() else {
            return;
        };
        for item in items {
            world.drop_item_stack(pos, item);
        }
    }

    fn load_additional(&self, nbt: &BorrowedNbtCompound<'_>) {
        // Convert to NbtCompound view for accessing methods
        let nbt_view: NbtCompoundView<'_, '_> = nbt.into();
        self.identity.load(nbt);
        let mut container = self.container.lock();
        container.items.fill(ItemStack::empty());
        if self.loot.load(nbt) {
            // Vanilla `RandomizableContainerBlockEntity.loadFromTag`: a stored
            // loot table replaces the item list, it does not accompany it.
            return;
        }

        // Load items from NBT using borrowed NBT for proper ItemStack parsing
        if let Some(items_list) = nbt_view.list("Items")
            && let Some(compounds) = items_list.compounds()
        {
            for compound in compounds {
                // Each item has a "Slot" byte and item data
                if let Some(slot) = compound.byte("Slot") {
                    let slot = slot as usize;
                    if slot < BARREL_SLOTS {
                        // Parse item directly from the borrowed compound
                        if let Some(item) = ItemStack::from_borrowed_compound(&compound) {
                            container.items[slot] = item;
                        }
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
        // Save items to NBT (only non-empty slots)
        let container = self.container.lock();
        let mut items: Vec<NbtCompound> = Vec::new();
        for (slot, item) in container.items.iter().enumerate() {
            if !item.is_empty() {
                // Use ItemStack's ToNbtTag implementation for proper component serialization
                if let NbtTag::Compound(mut item_nbt) = item.clone().to_nbt_tag() {
                    item_nbt.insert("Slot", slot as i8);
                    items.push(item_nbt);
                }
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
                "failed to collect the container component of the barrel at {:?}: {error}",
                self.get_block_pos()
            ),
        }
    }

    fn remove_components_from_tag(&self, nbt: &mut NbtCompound) {
        ContainerIdentity::remove_components_from_tag(nbt);
        ContainerLoot::remove_components_from_tag(nbt);
        while nbt.remove("Items").is_some() {}
    }

    fn get_update_tag(&self) -> Option<NbtCompound> {
        // Barrels don't need to send inventory to clients on chunk load
        // (unlike signs which display text)
        None
    }

    fn container_ref(&self) -> Option<ContainerRef> {
        Some(self.container_ref.clone())
    }
}

impl ContainerOpenersListener for BarrelBlockEntity {
    fn on_open(&self, world: &Arc<World>, pos: BlockPos, state: BlockStateId) {
        Self::play_sound(world, pos, state, true);
        Self::update_open_state(world, pos, state, true);
    }

    fn on_close(&self, world: &Arc<World>, pos: BlockPos, state: BlockStateId) {
        Self::play_sound(world, pos, state, false);
        Self::update_open_state(world, pos, state, false);
    }

    fn opener_count_changed(
        &self,
        _world: &Arc<World>,
        _pos: BlockPos,
        _state: BlockStateId,
        _previous: i32,
        _current: i32,
    ) {
        // Vanilla `BarrelBlockEntity` sends no block event for barrels.
    }

    fn is_own_container(&self, player: &Player) -> bool {
        player.has_container_open_for(&self.container_ref)
    }
}

impl Container for BarrelContainer {
    fn items(&self) -> &[ItemStack] {
        &self.items
    }

    fn items_mut(&mut self) -> &mut [ItemStack] {
        &mut self.items
    }

    fn get_container_size(&self) -> usize {
        BARREL_SLOTS
    }

    fn set_item(&mut self, slot: usize, mut stack: ItemStack) {
        if slot < BARREL_SLOTS {
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
    use std::io::Cursor;

    use simdnbt::borrow::read_compound as read_borrowed_compound;
    use text_components::TextComponent;
    use vecta_registry::{init_vanilla_registry, vanilla_blocks, vanilla_items};

    use super::*;

    fn test_barrel() -> BarrelBlockEntity {
        init_vanilla_registry();
        BarrelBlockEntity::new(
            Weak::new(),
            BlockPos::new(1, 2, 3),
            vanilla_blocks::BARREL.default_state(),
        )
    }

    /// Loads `CustomName`/`lock` the way `/data merge` or a placed block item
    /// would.
    fn load_identity(barrel: &BarrelBlockEntity, name: Option<&str>, lock_item: Option<&str>) {
        let mut nbt = NbtCompound::new();
        if let Some(name) = name {
            nbt.insert(
                "CustomName",
                TextComponent::plain(name.to_owned()).to_codec_nbt(),
            );
        }
        if let Some(lock_item) = lock_item {
            let mut lock = NbtCompound::new();
            lock.insert("items", lock_item);
            nbt.insert("lock", lock);
        }
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("identity NBT reads");
        barrel.load_additional(&borrowed);
    }

    #[test]
    fn set_item_limits_stack_to_vanilla_container_maximum() {
        let barrel = test_barrel();
        barrel
            .container
            .lock()
            .set_item(0, ItemStack::with_count(&vanilla_items::STONE, 100));

        assert_eq!(barrel.container.lock().get_item(0).count(), 64);
    }

    #[test]
    fn pre_remove_preserves_slots_for_existing_menu_references() {
        let barrel = test_barrel();
        barrel
            .container
            .lock()
            .set_item(0, ItemStack::new(&vanilla_items::STONE));

        barrel.pre_remove_side_effects(
            BlockPos::new(1, 2, 3),
            vanilla_blocks::BARREL.default_state(),
        );

        let container = barrel.container.lock();
        assert_eq!(container.items.len(), BARREL_SLOTS);
        assert!(container.items.iter().all(ItemStack::is_empty));
    }

    #[test]
    fn the_custom_name_and_lock_survive_an_nbt_round_trip() {
        let barrel = test_barrel();
        load_identity(&barrel, Some("Storage"), Some("minecraft:diamond"));

        let mut nbt = NbtCompound::new();
        barrel.save_additional(&mut nbt);
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("barrel NBT decodes");

        let loaded = BarrelBlockEntity::new(
            Weak::new(),
            BlockPos::new(1, 2, 3),
            vanilla_blocks::BARREL.default_state(),
        );
        loaded.load_additional(&borrowed);

        assert!(loaded.identity().is_locked());
        assert_eq!(
            loaded
                .identity()
                .display_name(|| TextComponent::plain("default")),
            TextComponent::plain("Storage"),
        );
    }

    #[test]
    fn a_loot_table_replaces_the_saved_items_until_it_is_rolled() {
        let barrel = test_barrel();
        let mut nbt = NbtCompound::new();
        nbt.insert("LootTable", "minecraft:chests/simple_dungeon");
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("loot NBT reads");
        barrel.load_additional(&borrowed);

        let mut pending = NbtCompound::new();
        barrel.save_additional(&mut pending);
        assert!(pending.get("LootTable").is_some());
        assert!(pending.get("Items").is_none());

        barrel.unpack_loot_table(None);

        let mut rolled = NbtCompound::new();
        barrel.save_additional(&mut rolled);
        assert!(rolled.get("LootTable").is_none());
        assert!(rolled.get("Items").is_some());
        assert!(
            barrel
                .container
                .lock()
                .items
                .iter()
                .any(|stack| !stack.is_empty()),
            "the dungeon table rolls at least one item"
        );
    }

    #[test]
    fn the_container_component_carries_the_items_into_a_placed_barrel() {
        let barrel = test_barrel();
        barrel
            .container
            .lock()
            .set_item(7, ItemStack::with_count(&vanilla_items::DIAMOND, 5));

        let mut components = DataComponentMap::new();
        barrel.collect_implicit_components(&mut components);
        let contents = components
            .get(CONTAINER)
            .expect("a barrel publishes its contents as a component");

        let placed = BarrelBlockEntity::new(
            Weak::new(),
            BlockPos::new(1, 2, 3),
            vanilla_blocks::BARREL.default_state(),
        );
        let mut item = ItemStack::new(&vanilla_items::BARREL);
        item.set(CONTAINER, contents);
        placed.apply_components_from_item(&item);

        let container = placed.container.lock();
        assert!(container.get_item(7).is(&vanilla_items::DIAMOND));
        assert_eq!(container.get_item(7).count(), 5);
    }
}
