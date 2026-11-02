//! The dispense behavior contract shared by dispensers and droppers.
//!
//! Vanilla keeps a `Map<Item, DispenseItemBehavior>` filled by
//! `DispenserBlock.registerBehavior`, falls back to `getDefaultDispenseMethod`
//! for items without an entry, and hands the leftover stack back so the block
//! entity can write it into the slot it took the item from. `DropperBlock`
//! replaces the whole lookup with one `DefaultDispenseItemBehavior`.

use std::ops::Deref;
use std::slice;
use std::sync::{Arc, OnceLock};

use glam::DVec3;
use rustc_hash::FxHashMap;
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::blocks::properties::Direction;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::items::ItemRef;
use vecta_registry::level_events;
use vecta_utils::axis::Axis;
use vecta_utils::random::Random as _;
use vecta_utils::{BlockPos, BlockStateId, Identifier};

use crate::inventory::lock::{ContainerLockGuard, ContainerRef};
use crate::world::World;

use super::super::dispenser_block::FACING;

/// Vanilla `DefaultDispenseItemBehavior.DEFAULT_ACCURACY`.
pub const DEFAULT_ACCURACY: f64 = 6.0;

/// Vanilla `DispenserBlock.getDispensePosition` scale.
pub const SPAWN_OFFSET: f64 = 0.7;

/// Vanilla `BlockSource`: everything a dispense behavior may read about the
/// block that is dispensing.
///
/// Vanilla also exposes the `DispenserBlockEntity`; behaviors that need it reach
/// it through [`BlockSource::container`] rather than carrying it here.
pub struct BlockSource<'a> {
    /// The level the dispense happens in.
    pub world: &'a Arc<World>,
    /// The block that dispenses.
    pub pos: BlockPos,
    /// The state of that block, which carries `FACING`.
    pub state: BlockStateId,
}

impl BlockSource<'_> {
    /// Vanilla `BlockSource.center`: the centre of the dispensing block.
    #[must_use]
    pub fn center(&self) -> DVec3 {
        DVec3::new(
            f64::from(self.pos.x()) + 0.5,
            f64::from(self.pos.y()) + 0.5,
            f64::from(self.pos.z()) + 0.5,
        )
    }

    /// Vanilla `BlockSource.state().getValue(DispenserBlock.FACING)`.
    #[must_use]
    pub fn facing(&self) -> Direction {
        self.state.get_value(FACING)
    }

    /// Vanilla `BlockSource.blockEntity()`, narrowed to the container capability
    /// `consumeWithRemainder` needs to store an empty bucket.
    #[must_use]
    pub fn container(&self) -> Option<ContainerRef> {
        ContainerRef::from_block_entity(self.world.get_block_entity(self.pos)?)
    }
}

/// Vanilla `DispenseItemBehavior`: consumes one item and returns what is left in
/// the slot.
pub trait DispenseItemBehavior: Send + Sync {
    /// Dispenses `stack`, returning the stack that stays in the slot.
    fn dispense(&self, source: &BlockSource<'_>, stack: ItemStack) -> ItemStack;
}

/// Vanilla `DefaultDispenseItemBehavior`: throws one item out of the block.
#[derive(Debug, Default)]
pub struct DefaultDispenseItemBehavior;

impl DispenseItemBehavior for DefaultDispenseItemBehavior {
    fn dispense(&self, source: &BlockSource<'_>, stack: ItemStack) -> ItemStack {
        let mut remaining = stack;
        let mut one = remaining.clone();
        one.set_count(1);
        remaining.shrink(1);
        Self::eject(source, one);
        remaining
    }
}

impl DefaultDispenseItemBehavior {
    /// Vanilla `DefaultDispenseItemBehavior.consumeWithRemainder`: take one item
    /// from `dispensed` and either leave `remainder` in the slot it emptied or
    /// store it in the dispenser.
    pub fn consume_with_remainder(
        source: &BlockSource<'_>,
        mut dispensed: ItemStack,
        remainder: ItemStack,
    ) -> ItemStack {
        dispensed.shrink(1);
        if dispensed.is_empty() {
            return remainder;
        }

        Self::add_to_inventory_or_dispense(source, remainder);
        dispensed
    }

    /// Vanilla `DefaultDispenseItemBehavior.addToInventoryOrDispense`: store the
    /// stack in the dispenser, ejecting whatever does not fit.
    fn add_to_inventory_or_dispense(source: &BlockSource<'_>, mut stack: ItemStack) {
        if let Some(container) = source.container() {
            let id = container.container_id();
            let mut guard = ContainerLockGuard::lock_all(slice::from_ref(&container));
            if let Some(locked) = guard.get_mut(id) {
                locked.add(&mut stack);
            }
        }

        if !stack.is_empty() {
            Self::eject(source, stack);
        }
    }

    /// Vanilla `DefaultDispenseItemBehavior.playSound` +
    /// `spawnItem` + `playAnimation`: one item entity in front of the block.
    pub fn eject(source: &BlockSource<'_>, item: ItemStack) {
        Self::spawn_item(source, item);
        source
            .world
            .level_event(level_events::SOUND_DISPENSER_DISPENSE, source.pos, 0, None);
        source.world.level_event(
            level_events::PARTICLES_SHOOT_SMOKE,
            source.pos,
            source.facing().get_3d_data_value(),
            None,
        );
    }

    /// Vanilla `DefaultDispenseItemBehavior.spawnItem`: one item entity in
    /// front of the block, without the sound and animation.
    pub fn spawn_item(source: &BlockSource<'_>, item: ItemStack) {
        // Vanilla `DispenserBlock.getDispensePosition` reads the facing back out
        // of the block state instead of trusting the caller.
        let facing = source.facing();
        let (dx, _, dz) = facing.offset();
        let spawn = source.center() + DVec3::from(facing.offset_vec()) * SPAWN_OFFSET;
        // Vanilla lowers the spawn point by 2/16 on the Y axis and 2.5/16 on the
        // horizontal axes so the item does not clip the block face.
        let y = spawn.y
            - if facing.get_axis() == Axis::Y {
                0.125
            } else {
                0.15625
            };

        // Vanilla `spawnItem` draws from `level.getRandom()`, so the level stream
        // advances: `nextDouble` for the power, then one `triangle` per axis.
        let velocity = source.world.random().with_level(|rng| {
            let pow = rng.next_f64() * 0.1 + 0.2;
            let deviation = 0.017_227_5 * DEFAULT_ACCURACY;
            DVec3::new(
                rng.triangle(f64::from(dx) * pow, deviation),
                rng.triangle(0.2, deviation),
                rng.triangle(f64::from(dz) * pow, deviation),
            )
        });

        source
            .world
            .spawn_item_with_velocity(DVec3::new(spawn.x, y, spawn.z), item, velocity);
    }
}

/// Vanilla `DispenserBlock.DISPENSER_REGISTRY`: item → behavior, with
/// [`DefaultDispenseItemBehavior`] for anything unregistered.
///
/// Items register themselves through
/// [`register_dispense_behaviors`](crate::behavior::register_dispense_behaviors)
/// rather than being listed here, so a new behavior never has to touch the
/// dispense tick.
#[derive(Default)]
pub struct DispenseBehaviorRegistry {
    behaviors: FxHashMap<Identifier, Box<dyn DispenseItemBehavior>>,
}

impl DispenseBehaviorRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Vanilla `DispenserBlock.registerBehavior`.
    pub fn register(&mut self, item: ItemRef, behavior: impl DispenseItemBehavior + 'static) {
        self.behaviors.insert(item.key.clone(), Box::new(behavior));
    }

    /// Returns the behavior registered for `item`, if any.
    #[must_use]
    pub fn get(&self, item: ItemRef) -> Option<&dyn DispenseItemBehavior> {
        self.behaviors.get(&item.key).map(Box::as_ref)
    }
}

/// Vanilla `DispenserBlock.DEFAULT_BEHAVIOR`.
pub static DEFAULT_BEHAVIOR: DefaultDispenseItemBehavior = DefaultDispenseItemBehavior;

/// The shared, lazily built dispense registry.
pub(crate) struct DispenseBehaviorLock(pub(crate) OnceLock<DispenseBehaviorRegistry>);

impl Deref for DispenseBehaviorLock {
    type Target = DispenseBehaviorRegistry;

    fn deref(&self) -> &Self::Target {
        self.0.get().expect("Dispense behaviors not initialized")
    }
}

#[cfg(test)]
mod tests {
    use vecta_registry::{init_vanilla_registry, vanilla_items};

    use super::*;

    /// A behavior that hands the stack straight back, so a test can prove the
    /// lookup resolves to the registered entry.
    struct Probe;

    impl DispenseItemBehavior for Probe {
        fn dispense(&self, _source: &BlockSource<'_>, stack: ItemStack) -> ItemStack {
            stack
        }
    }

    #[test]
    fn the_registry_only_answers_for_registered_items() {
        init_vanilla_registry();
        let mut registry = DispenseBehaviorRegistry::new();
        assert!(
            registry.get(&vanilla_items::STONE).is_none(),
            "vanilla `getDispenseMethod` falls back to `getDefaultDispenseMethod` for an unregistered item"
        );

        registry.register(&vanilla_items::STONE, Probe);
        assert!(
            registry.get(&vanilla_items::STONE).is_some(),
            "a registered item resolves to its own behavior"
        );
        assert!(
            registry.get(&vanilla_items::DIAMOND).is_none(),
            "registration is per item, not global"
        );
    }
}
