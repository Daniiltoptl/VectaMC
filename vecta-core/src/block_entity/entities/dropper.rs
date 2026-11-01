//! Dropper block entity implementation.
//!
//! A dropper is a dispenser with its own block entity type: vanilla
//! `DropperBlockEntity` extends `DispenserBlockEntity` and only changes the type
//! and the default name, so this wrapper delegates the container, the loot table
//! and the custom name to [`DispenserBlockEntity`].

use std::sync::Weak;

use simdnbt::borrow::BaseNbtCompound as BorrowedNbtCompound;
use simdnbt::owned::NbtCompound;
use vecta_registry::data_components::DataComponentMap;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::vanilla_block_entity_types;
use vecta_utils::{BlockPos, BlockStateId, DowncastType, DowncastTypeKey, Identifier};

use crate::block_entity::container_identity::ContainerIdentity;
use crate::block_entity::entities::dispenser::{DISPENSER_SLOTS, DispenserBlockEntity};
use crate::block_entity::{BlockEntity, BlockEntityBase};
use crate::inventory::lock::ContainerRef;
use crate::player::Player;
use crate::world::World;

/// Vanilla `DispenserBlockEntity.CONTAINER_SIZE`: a dropper has the same nine
/// slots as the dispenser it extends.
pub const DROPPER_SLOTS: usize = DISPENSER_SLOTS;

/// Dropper block entity.
pub struct DropperBlockEntity {
    inner: DispenserBlockEntity,
}

// SAFETY: This key is owned by Vecta and uniquely identifies `DropperBlockEntity`.
unsafe impl DowncastType for DropperBlockEntity {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:block_entity/dropper");
}

impl DropperBlockEntity {
    /// Creates a new dropper block entity.
    #[must_use]
    pub fn new(level: Weak<World>, pos: BlockPos, state: BlockStateId) -> Self {
        Self {
            inner: DispenserBlockEntity::with_type(
                &vanilla_block_entity_types::DROPPER,
                level,
                pos,
                state,
            ),
        }
    }

    /// Returns the custom name and lock of this dropper.
    #[must_use]
    pub const fn identity(&self) -> &ContainerIdentity {
        self.inner.identity()
    }

    /// Vanilla `RandomizableContainerBlockEntity.canOpen`.
    #[must_use]
    pub fn can_open(&self, player: &Player) -> bool {
        self.inner.can_open(player)
    }

    /// Rolls this dropper's pending loot table, if any.
    pub fn unpack_loot_table(&self, player: Option<&Player>) {
        self.inner.unpack_loot_table(player);
    }

    /// Vanilla `DispenserBlockEntity.getRandomSlot`.
    pub fn get_random_slot(&self, next_int: impl FnMut(i32) -> i32) -> Option<usize> {
        self.inner.get_random_slot(next_int)
    }

    /// The shared container state, used by the dropper's own tests.
    #[cfg(test)]
    pub(crate) const fn inner(&self) -> &DispenserBlockEntity {
        &self.inner
    }
}

impl BlockEntity for DropperBlockEntity {
    fn base(&self) -> &BlockEntityBase {
        self.inner.base()
    }

    fn pre_remove_side_effects(&self, pos: BlockPos, state: BlockStateId) {
        self.inner.pre_remove_side_effects(pos, state);
    }

    fn load_additional(&self, nbt: &BorrowedNbtCompound<'_>) {
        self.inner.load_additional(nbt);
    }

    fn save_additional(&self, nbt: &mut NbtCompound) {
        self.inner.save_additional(nbt);
    }

    fn apply_implicit_components(&self, item: &ItemStack) -> Vec<Identifier> {
        self.inner.apply_implicit_components(item)
    }

    fn collect_implicit_components(&self, components: &mut DataComponentMap) {
        self.inner.collect_implicit_components(components);
    }

    fn remove_components_from_tag(&self, nbt: &mut NbtCompound) {
        self.inner.remove_components_from_tag(nbt);
    }

    fn container_ref(&self) -> Option<ContainerRef> {
        self.inner.container_ref()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use simdnbt::borrow::read_compound as read_borrowed_compound;
    use simdnbt::owned::NbtCompound;
    use vecta_registry::{init_vanilla_registry, vanilla_blocks, vanilla_items};

    use super::*;
    use crate::inventory::container::Container as _;

    fn test_dropper() -> DropperBlockEntity {
        init_vanilla_registry();
        DropperBlockEntity::new(
            Weak::new(),
            BlockPos::new(1, 2, 3),
            vanilla_blocks::DROPPER.default_state(),
        )
    }

    #[test]
    fn a_dropper_uses_the_dropper_block_entity_type() {
        let dropper = test_dropper();
        assert_eq!(
            dropper.get_type().key,
            vanilla_block_entity_types::DROPPER.key,
            "vanilla `DropperBlockEntity` only changes the type"
        );
        assert_eq!(DROPPER_SLOTS, DISPENSER_SLOTS);
    }

    #[test]
    fn set_item_limits_a_stack_to_the_item_maximum() {
        let dropper = test_dropper();
        dropper
            .inner()
            .container()
            .lock()
            .set_item(0, ItemStack::with_count(&vanilla_items::STONE, 100));

        assert_eq!(dropper.inner().container().lock().get_item(0).count(), 64);
    }

    #[test]
    fn an_empty_dropper_has_no_random_slot() {
        let dropper = test_dropper();

        assert_eq!(dropper.get_random_slot(|_| 0), None);
    }

    #[test]
    fn the_random_slot_choice_is_reservoir_sampling() {
        let dropper = test_dropper();
        {
            let mut container = dropper.inner().container().lock();
            container.set_item(1, ItemStack::new(&vanilla_items::STONE));
            container.set_item(4, ItemStack::new(&vanilla_items::DIAMOND));
        }

        // `nextInt(1) == 0` always replaces the first candidate, so the last
        // non-empty slot wins; returning the largest legal value for every
        // larger bound never replaces, which keeps the first one.
        let mut bounds = Vec::new();
        let last = dropper.get_random_slot(|bound| {
            bounds.push(bound);
            0
        });
        assert_eq!(last, Some(4));
        assert_eq!(bounds, [1, 2], "only non-empty slots widen the odds");

        let mut bounds = Vec::new();
        let first = dropper.get_random_slot(|bound| {
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
        let dropper = test_dropper();
        dropper
            .inner()
            .container()
            .lock()
            .set_item(8, ItemStack::with_count(&vanilla_items::DIAMOND, 3));

        let mut nbt = NbtCompound::new();
        dropper.save_additional(&mut nbt);
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed = read_borrowed_compound(&mut Cursor::new(bytes.as_slice()))
            .expect("dropper NBT decodes");

        let loaded = test_dropper();
        loaded.load_additional(&borrowed);

        assert_eq!(loaded.inner().container().lock().get_item(8).count(), 3);
        assert_eq!(
            loaded.inner().container().lock().items().len(),
            DROPPER_SLOTS
        );
    }

    #[test]
    fn pre_remove_clears_every_slot() {
        let dropper = test_dropper();
        dropper
            .inner()
            .container()
            .lock()
            .set_item(0, ItemStack::new(&vanilla_items::STONE));

        dropper.pre_remove_side_effects(
            BlockPos::new(1, 2, 3),
            vanilla_blocks::DROPPER.default_state(),
        );

        let container = dropper.inner().container().lock();
        assert!(container.items().iter().all(ItemStack::is_empty));
    }
}
