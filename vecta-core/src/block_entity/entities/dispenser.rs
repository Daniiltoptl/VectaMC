//! Dispenser block entity implementation.
//!
//! A dispenser holds nine slots and, on every redstone pulse, hands one item
//! from a randomly chosen non-empty slot to the dispense behavior registered for
//! it. Vanilla implements this as `DispenserBlockEntity`, and the dropper is a
//! subclass that only changes the block entity type and the default name, so the
//! shared state lives here and [`DropperBlockEntity`] wraps it.
//!
//! [`DropperBlockEntity`]: crate::block_entity::entities::DropperBlockEntity

use std::mem;
use std::sync::{Arc, Weak};

use simdnbt::ToNbtTag;
use simdnbt::borrow::{BaseNbtCompound as BorrowedNbtCompound, NbtCompound as NbtCompoundView};
use simdnbt::owned::{NbtCompound, NbtList, NbtTag};
use vecta_registry::block_entity_type::BlockEntityTypeRef;
use vecta_registry::data_components::DataComponentMap;
use vecta_registry::data_components::vanilla_components::CONTAINER;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::item_stack_template::ItemStackTemplate;
use vecta_registry::vanilla_block_entity_types;
use vecta_utils::{
    BlockPos, BlockStateId, DowncastType, DowncastTypeKey, Identifier, locks::SyncMutex,
};

use crate::block_entity::container_component::{
    apply_container_items, collect_container_component,
};
use crate::block_entity::container_identity::ContainerIdentity;
use crate::block_entity::container_loot::ContainerLoot;
use crate::block_entity::{BlockEntity, BlockEntityBase};
use crate::entity::Entity;
use crate::inventory::container::Container;
use crate::inventory::lock::{ContainerRef, SharedContainer};
use crate::player::Player;
use crate::world::World;

/// Vanilla `DispenserBlockEntity.CONTAINER_SIZE`.
pub const DISPENSER_SLOTS: usize = 9;

/// The nine-slot container behind a dispenser or a dropper.
pub struct DispenserContainer {
    items: Vec<ItemStack>,
}

// SAFETY: This key is owned by Vecta and uniquely identifies the independently
// lockable inventory data shared by the dispenser and the dropper.
unsafe impl DowncastType for DispenserContainer {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:container/dispenser");
}

/// Dispenser block entity: the container, custom name and pending loot of a
/// dispenser. A dropper reuses this type with the `DROPPER` block entity type.
pub struct DispenserBlockEntity {
    base: Arc<BlockEntityBase>,
    container: Arc<SyncMutex<DispenserContainer>>,
    container_ref: ContainerRef,
    identity: ContainerIdentity,
    loot: ContainerLoot,
}

// SAFETY: This key is owned by Vecta and uniquely identifies
// `DispenserBlockEntity`.
unsafe impl DowncastType for DispenserBlockEntity {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:block_entity/dispenser");
}

impl DispenserBlockEntity {
    /// Creates a new dispenser block entity.
    #[must_use]
    pub fn new(level: Weak<World>, pos: BlockPos, state: BlockStateId) -> Self {
        Self::with_type(&vanilla_block_entity_types::DISPENSER, level, pos, state)
    }

    /// Creates the shared container with an explicit block entity type, which
    /// is how the dropper reuses this implementation.
    #[must_use]
    pub fn with_type(
        block_entity_type: BlockEntityTypeRef,
        level: Weak<World>,
        pos: BlockPos,
        state: BlockStateId,
    ) -> Self {
        let base = Arc::new(BlockEntityBase::new(block_entity_type, level, pos, state));
        let container = Arc::new(SyncMutex::new(DispenserContainer {
            items: vec![ItemStack::empty(); DISPENSER_SLOTS],
        }));
        let shared_container: SharedContainer =
            Arc::<SyncMutex<DispenserContainer>>::clone(&container);
        Self {
            container_ref: ContainerRef::owned_by_block_entity(shared_container, Arc::clone(&base)),
            base,
            container,
            identity: ContainerIdentity::new(),
            loot: ContainerLoot::new(),
        }
    }

    /// Returns the custom name and lock of this container.
    #[must_use]
    pub const fn identity(&self) -> &ContainerIdentity {
        &self.identity
    }

    /// Vanilla `RandomizableContainerBlockEntity.canOpen`: a container that still
    /// has to roll its loot refuses spectators, then the lock decides.
    #[must_use]
    pub fn can_open(&self, player: &Player) -> bool {
        (!self.loot.is_randomized() || !player.is_spectator()) && self.identity.can_open(player)
    }

    /// Rolls this container's pending loot table, if any.
    pub fn unpack_loot_table(&self, player: Option<&Player>) {
        self.loot.unpack(
            self.base.level().as_deref(),
            self.get_block_pos(),
            &mut self.container.lock().items,
            player,
        );
    }

    /// Vanilla `DispenserBlockEntity.getRandomSlot`: reservoir sampling over the
    /// non-empty slots, so every non-empty slot is equally likely.
    ///
    /// `next_int` is the caller's `random.nextInt(bound)`; taking it as a
    /// closure keeps the choice testable without a global random source.
    pub fn get_random_slot(&self, mut next_int: impl FnMut(i32) -> i32) -> Option<usize> {
        self.unpack_loot_table(None);
        let container = self.container.lock();
        let mut chosen = None;
        let mut odds = 1;
        for (slot, stack) in container.items.iter().enumerate() {
            if !stack.is_empty() && next_int(odds) == 0 {
                chosen = Some(slot);
            }
            if !stack.is_empty() {
                odds += 1;
            }
        }
        chosen
    }

    /// Drops every item this container still holds.
    fn drop_contents(&self, pos: BlockPos) {
        self.unpack_loot_table(None);
        let items = {
            let mut container = self.container.lock();
            mem::replace(
                &mut container.items,
                vec![ItemStack::empty(); DISPENSER_SLOTS],
            )
        };
        let Some(world) = self.base.level() else {
            return;
        };
        for item in items {
            world.drop_item_stack(pos, item);
        }
    }

    /// Vanilla `DispenserBlockEntity.loadAdditional`.
    fn load(&self, nbt: &BorrowedNbtCompound<'_>) {
        let nbt_view: NbtCompoundView<'_, '_> = nbt.into();
        self.identity.load(nbt);
        let mut container = self.container.lock();
        container.items.fill(ItemStack::empty());
        if self.loot.load(nbt) {
            return;
        }
        if let Some(items_list) = nbt_view.list("Items")
            && let Some(compounds) = items_list.compounds()
        {
            for compound in compounds {
                if let Some(slot) = compound.byte("Slot") {
                    let slot = slot as usize;
                    if slot < DISPENSER_SLOTS
                        && let Some(item) = ItemStack::from_borrowed_compound(&compound)
                    {
                        container.items[slot] = item;
                    }
                }
            }
        }
    }

    /// Vanilla `DispenserBlockEntity.saveAdditional`.
    fn save(&self, nbt: &mut NbtCompound) {
        self.identity.save(nbt);
        if !self.loot.save(nbt) {
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
    }

    /// Vanilla `BlockEntity.applyComponentsFromItemStack` for a container item.
    fn apply_components(&self, item: &ItemStack) -> Vec<Identifier> {
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

    /// Vanilla `BlockEntity.collectComponents`.
    fn collect_components(&self, components: &mut DataComponentMap) {
        self.identity.collect_implicit_components(components);
        self.loot.collect_implicit_components(components);
        let contents = collect_container_component(&self.container.lock().items);
        match contents {
            Ok(contents) => components.set(CONTAINER, Some(contents)),
            Err(error) => log::warn!(
                "failed to collect the container component at {:?}: {error}",
                self.get_block_pos()
            ),
        }
    }
}

impl BlockEntity for DispenserBlockEntity {
    fn base(&self) -> &BlockEntityBase {
        &self.base
    }

    fn pre_remove_side_effects(&self, pos: BlockPos, _state: BlockStateId) {
        self.drop_contents(pos);
    }

    fn load_additional(&self, nbt: &BorrowedNbtCompound<'_>) {
        self.load(nbt);
    }

    fn save_additional(&self, nbt: &mut NbtCompound) {
        self.save(nbt);
    }

    fn apply_implicit_components(&self, item: &ItemStack) -> Vec<Identifier> {
        self.apply_components(item)
    }

    fn collect_implicit_components(&self, components: &mut DataComponentMap) {
        self.collect_components(components);
    }

    fn remove_components_from_tag(&self, nbt: &mut NbtCompound) {
        ContainerIdentity::remove_components_from_tag(nbt);
        ContainerLoot::remove_components_from_tag(nbt);
        while nbt.remove("Items").is_some() {}
    }

    fn container_ref(&self) -> Option<ContainerRef> {
        Some(self.container_ref.clone())
    }
}

impl Container for DispenserContainer {
    fn items(&self) -> &[ItemStack] {
        &self.items
    }

    fn items_mut(&mut self) -> &mut [ItemStack] {
        &mut self.items
    }

    fn get_container_size(&self) -> usize {
        DISPENSER_SLOTS
    }

    fn set_item(&mut self, slot: usize, mut stack: ItemStack) {
        if slot < DISPENSER_SLOTS {
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

/// Shared accessors the dropper wrapper needs from its inner dispenser entity.
impl DispenserBlockEntity {
    /// The container behind this entity, for tests of both block entities.
    #[cfg(test)]
    pub(crate) const fn container(&self) -> &Arc<SyncMutex<DispenserContainer>> {
        &self.container
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use simdnbt::borrow::read_compound as read_borrowed_compound;
    use vecta_registry::{init_vanilla_registry, vanilla_blocks, vanilla_items};

    use super::*;

    fn test_dispenser() -> DispenserBlockEntity {
        init_vanilla_registry();
        DispenserBlockEntity::new(
            Weak::new(),
            BlockPos::new(1, 2, 3),
            vanilla_blocks::DISPENSER.default_state(),
        )
    }

    #[test]
    fn a_dispenser_has_nine_slots_of_its_own_type() {
        let dispenser = test_dispenser();
        assert_eq!(
            dispenser.get_type().key,
            vanilla_block_entity_types::DISPENSER.key
        );
        assert_eq!(dispenser.container().lock().items.len(), DISPENSER_SLOTS);
    }

    #[test]
    fn set_item_limits_a_stack_to_the_item_maximum() {
        let dispenser = test_dispenser();
        dispenser
            .container()
            .lock()
            .set_item(0, ItemStack::with_count(&vanilla_items::STONE, 100));

        assert_eq!(dispenser.container().lock().get_item(0).count(), 64);
    }

    #[test]
    fn an_empty_dispenser_has_no_random_slot() {
        let dispenser = test_dispenser();

        assert_eq!(dispenser.get_random_slot(|_| 0), None);
    }

    #[test]
    fn the_random_slot_choice_is_reservoir_sampling() {
        let dispenser = test_dispenser();
        {
            let mut container = dispenser.container().lock();
            container.set_item(1, ItemStack::new(&vanilla_items::STONE));
            container.set_item(4, ItemStack::new(&vanilla_items::DIAMOND));
        }

        // `nextInt(1) == 0` always replaces the first candidate, so the last
        // non-empty slot wins; returning the largest legal value for every
        // larger bound never replaces, which keeps the first one.
        let mut bounds = Vec::new();
        let last = dispenser.get_random_slot(|bound| {
            bounds.push(bound);
            0
        });
        assert_eq!(last, Some(4));
        assert_eq!(bounds, [1, 2], "only non-empty slots widen the odds");

        let mut bounds = Vec::new();
        let first = dispenser.get_random_slot(|bound| {
            bounds.push(bound);
            bound - 1
        });
        assert_eq!(
            first,
            Some(1),
            "a bound that never returns zero keeps the first"
        );
        assert_eq!(bounds, [1, 2]);
    }

    #[test]
    fn the_contents_survive_an_nbt_round_trip() {
        let dispenser = test_dispenser();
        dispenser
            .container()
            .lock()
            .set_item(8, ItemStack::with_count(&vanilla_items::DIAMOND, 3));

        let mut nbt = NbtCompound::new();
        dispenser.save_additional(&mut nbt);
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed = read_borrowed_compound(&mut Cursor::new(bytes.as_slice()))
            .expect("dispenser NBT decodes");

        let loaded = test_dispenser();
        loaded.load_additional(&borrowed);

        assert_eq!(loaded.container().lock().get_item(8).count(), 3);
        assert_eq!(loaded.container().lock().items.len(), DISPENSER_SLOTS);
    }

    #[test]
    fn pre_remove_clears_every_slot() {
        let dispenser = test_dispenser();
        dispenser
            .container()
            .lock()
            .set_item(0, ItemStack::new(&vanilla_items::STONE));

        dispenser.pre_remove_side_effects(
            BlockPos::new(1, 2, 3),
            vanilla_blocks::DISPENSER.default_state(),
        );

        let container = dispenser.container().lock();
        assert!(container.items.iter().all(ItemStack::is_empty));
    }
}
