//! Hopper block entity implementation.
//!
//! A hopper holds five slots and, on every tick it is not cooling down, moves at
//! most one item: first it pushes an item into the container it points at, then
//! it pulls one from the container above it or from the item entities resting in
//! its upper half. Both moves share the same eight-tick cooldown, which is
//! persisted so a chunk reload resumes where the hopper left off.

use std::{
    mem, slice,
    sync::{
        Arc, Weak,
        atomic::{AtomicI32, AtomicI64, Ordering},
    },
};

use simdnbt::ToNbtTag;
use simdnbt::borrow::{BaseNbtCompound as BorrowedNbtCompound, NbtCompound as NbtCompoundView};
use simdnbt::owned::{NbtCompound, NbtList, NbtTag};
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::blocks::properties::{
    BlockStateProperties, BoolProperty, Direction, EnumProperty,
};
use vecta_registry::blocks::shapes::is_shape_full_block;
use vecta_registry::data_components::DataComponentMap;
use vecta_registry::data_components::vanilla_components::CONTAINER;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::item_stack_template::ItemStackTemplate;
use vecta_registry::vanilla_block_entity_types;
use vecta_registry::vanilla_block_tags::BlockTag;
use vecta_utils::{
    BlockPos, BlockStateId, Downcast as _, DowncastType, DowncastTypeKey, Identifier, WorldAabb,
    locks::SyncMutex,
};

use crate::behavior::blocks::{ChestBlock, ChestPair};
use crate::behavior::{BLOCK_BEHAVIORS, BlockCollisionContext};
use crate::block_entity::container_component::{
    apply_container_items, collect_container_component,
};
use crate::block_entity::container_identity::ContainerIdentity;
use crate::block_entity::container_loot::ContainerLoot;
use crate::block_entity::entities::{BarrelBlockEntity, ChestBlockEntity, ShulkerBoxBlockEntity};
use crate::block_entity::{BlockEntity, BlockEntityBase, SharedBlockEntity};
use crate::entity::entities::ItemEntity;
use crate::entity::{Entity, RemovalReason, SharedEntity};
use crate::inventory::container::Container;
use crate::inventory::lock::{ContainerId, ContainerLockGuard, ContainerRef, SharedContainer};
use crate::player::Player;
use crate::world::{LevelReader, World};

/// Number of slots in a hopper.
pub const HOPPER_SLOTS: usize = 5;

/// Vanilla `HopperBlockEntity.MOVE_ITEM_SPEED`: ticks a hopper waits after a
/// successful transfer.
pub const MOVE_ITEM_SPEED: i32 = 8;

/// Vanilla `HopperBlockEntity.NO_COOLDOWN_TIME`.
pub const NO_COOLDOWN_TIME: i32 = -1;

/// Lowest block-local Y of `Hopper.SUCK_AABB`
/// (`Block.column(16.0, 11.0, 32.0)`).
const SUCK_MIN_Y: f64 = 11.0 / 16.0;

/// Highest block-local Y of `Hopper.SUCK_AABB`.
const SUCK_MAX_Y: f64 = 2.0;

const FACING: &EnumProperty<Direction> = &BlockStateProperties::FACING_HOPPER;
const ENABLED: &BoolProperty = &BlockStateProperties::ENABLED;

/// Hopper block entity.
pub struct HopperBlockEntity {
    base: Arc<BlockEntityBase>,
    container: Arc<SyncMutex<HopperContainer>>,
    container_ref: ContainerRef,
    identity: ContainerIdentity,
    loot: ContainerLoot,
    /// Vanilla `HopperBlockEntity.cooldownTime`.
    cooldown_time: AtomicI32,
    /// Vanilla `HopperBlockEntity.tickedGameTime`.
    ticked_game_time: AtomicI64,
}

struct HopperContainer {
    items: Vec<ItemStack>,
}

// SAFETY: This key is owned by Vecta and uniquely identifies `HopperBlockEntity`.
unsafe impl DowncastType for HopperBlockEntity {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:block_entity/hopper");
}

// SAFETY: This key is owned by Vecta and uniquely identifies the independently
// lockable inventory data used by a hopper block entity.
unsafe impl DowncastType for HopperContainer {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:container/hopper");
}

impl HopperBlockEntity {
    /// Creates a new hopper block entity.
    #[must_use]
    pub fn new(level: Weak<World>, pos: BlockPos, state: BlockStateId) -> Self {
        let base = Arc::new(BlockEntityBase::new(
            &vanilla_block_entity_types::HOPPER,
            level,
            pos,
            state,
        ));
        let container = Arc::new(SyncMutex::new(HopperContainer {
            items: vec![ItemStack::empty(); HOPPER_SLOTS],
        }));
        let shared_container: SharedContainer =
            Arc::<SyncMutex<HopperContainer>>::clone(&container);
        Self {
            container_ref: ContainerRef::owned_by_block_entity(shared_container, Arc::clone(&base)),
            base,
            container,
            identity: ContainerIdentity::new(),
            loot: ContainerLoot::new(),
            cooldown_time: AtomicI32::new(NO_COOLDOWN_TIME),
            ticked_game_time: AtomicI64::new(0),
        }
    }

    /// Returns the custom name and lock of this hopper.
    #[must_use]
    pub const fn identity(&self) -> &ContainerIdentity {
        &self.identity
    }

    /// Vanilla `RandomizableContainerBlockEntity.canOpen`: a hopper that still
    /// has to roll its loot refuses spectators, then the lock decides.
    #[must_use]
    pub fn can_open(&self, player: &Player) -> bool {
        (!self.loot.is_randomized() || !player.is_spectator()) && self.identity.can_open(player)
    }

    /// Rolls this hopper's pending loot table, if any.
    ///
    /// Vanilla `RandomizableContainerBlockEntity` does this on every contents
    /// read, so opening, a comparator read and a transfer all see the rolled
    /// contents while only the first one generates them.
    pub fn unpack_loot_table(&self, player: Option<&Player>) {
        self.loot.unpack(
            self.get_block_pos(),
            &mut self.container.lock().items,
            player,
        );
    }

    /// Vanilla `HopperBlockEntity.cooldownTime`.
    #[must_use]
    pub fn cooldown_time(&self) -> i32 {
        self.cooldown_time.load(Ordering::Relaxed)
    }

    /// Vanilla `HopperBlockEntity.setCooldown`.
    pub fn set_cooldown(&self, time: i32) {
        self.cooldown_time.store(time, Ordering::Relaxed);
    }

    /// Vanilla `HopperBlockEntity.isOnCooldown`.
    #[must_use]
    pub fn is_on_cooldown(&self) -> bool {
        self.cooldown_time() > 0
    }

    /// Vanilla `HopperBlockEntity.isOnCustomCooldown`: a cooldown longer than the
    /// transfer interval was set by another hopper, so the empty-slot shortcut
    /// must not shorten it again.
    #[must_use]
    pub fn is_on_custom_cooldown(&self) -> bool {
        self.cooldown_time() > MOVE_ITEM_SPEED
    }

    /// Vanilla `HopperBlockEntity.tickedGameTime`.
    #[must_use]
    pub fn ticked_game_time(&self) -> i64 {
        self.ticked_game_time.load(Ordering::Relaxed)
    }

    /// Vanilla `HopperBlockEntity.inventoryFull`: every slot is non-empty and at
    /// its item's maximum stack size.
    #[must_use]
    pub fn inventory_full(&self) -> bool {
        self.container
            .lock()
            .items
            .iter()
            .all(|stack| !stack.is_empty() && stack.count() == stack.max_stack_size())
    }

    /// Vanilla `HopperBlockEntity.isEmpty`, which unpacks the loot table first.
    fn inventory_empty(&self) -> bool {
        self.unpack_loot_table(None);
        self.container.lock().items.iter().all(ItemStack::is_empty)
    }

    /// Vanilla `HopperBlockEntity.pushItemsTick`, registered as the hopper's
    /// block-entity ticker.
    ///
    /// The cooldown counts down every tick; a hopper at zero tries to move one
    /// item out and then one item in, and a successful move restarts the
    /// cooldown.
    pub(crate) fn push_items_tick(
        world: &Arc<World>,
        pos: BlockPos,
        state: BlockStateId,
        block_entity: &dyn BlockEntity,
    ) {
        let Some(hopper) = block_entity.downcast_ref::<HopperBlockEntity>() else {
            return;
        };
        hopper.set_cooldown(hopper.cooldown_time() - 1);
        hopper
            .ticked_game_time
            .store(world.game_time(), Ordering::Relaxed);
        if hopper.is_on_cooldown() {
            return;
        }
        hopper.set_cooldown(0);
        hopper.try_move_items(world, pos, state, || Self::suck_in_items(world, hopper));
    }

    /// Vanilla `HopperBlockEntity.tryMoveItems`: one push, then one pull, under a
    /// disabled-state and cooldown gate.
    ///
    /// `entityInside` calls this directly, so the gate is repeated here rather
    /// than relying on [`Self::push_items_tick`].
    fn try_move_items(
        &self,
        world: &Arc<World>,
        pos: BlockPos,
        state: BlockStateId,
        action: impl FnOnce() -> bool,
    ) -> bool {
        if self.is_on_cooldown() || !state.get_value(ENABLED) {
            return false;
        }

        let mut changed = false;
        if !self.inventory_empty() {
            changed = Self::eject_items(world, pos, state, self);
        }
        if !self.inventory_full() {
            changed |= action();
        }

        if changed {
            self.set_cooldown(MOVE_ITEM_SPEED);
            self.set_changed();
            return true;
        }
        false
    }

    /// Vanilla `HopperBlockEntity.ejectItems`: pushes the first movable slot into
    /// the container the hopper points at.
    fn eject_items(
        world: &Arc<World>,
        pos: BlockPos,
        state: BlockStateId,
        hopper: &HopperBlockEntity,
    ) -> bool {
        let facing = state.get_value(FACING);
        let Some(target) = container_at(world, pos.relative(facing)) else {
            return false;
        };
        // The face items enter through is the one facing the hopper.
        let direction = facing.opposite();
        let Some(hopper_ref) = hopper.container_ref() else {
            return false;
        };
        let hopper_id = hopper_ref.container_id();

        let mut refs = Vec::with_capacity(target.parts.len() + 1);
        refs.push(hopper_ref);
        refs.extend(target.parts.iter().cloned());
        let mut guard = ContainerLockGuard::lock_all(&refs);

        if target.is_full(&guard, direction) {
            return false;
        }

        for slot in 0..HOPPER_SLOTS {
            let Some(stack) = guard
                .get(hopper_id)
                .map(|container| container.get_item(slot).clone())
            else {
                return false;
            };
            if stack.is_empty() {
                continue;
            }
            let original_count = stack.count();
            let removed = guard
                .get_mut(hopper_id)
                .map_or_else(ItemStack::empty, |container| container.remove_item(slot, 1));
            if removed.is_empty() {
                return false;
            }

            let remaining = add_item(
                &mut guard,
                &target,
                removed,
                Some(direction),
                Some(hopper.ticked_game_time()),
                target.hopper(),
            );
            if remaining.is_empty() {
                target.mark_changed(&mut guard);
                return true;
            }

            restore_source(&mut guard, hopper_id, slot, stack, original_count);
        }

        false
    }

    /// Vanilla `HopperBlockEntity.suckInItems`: pulls one item from the container
    /// above, or else from the item entities in `Hopper.SUCK_AABB`.
    fn suck_in_items(world: &Arc<World>, hopper: &HopperBlockEntity) -> bool {
        let above = hopper.get_block_pos().above();
        let state = world.get_block_state(above);

        if let Some(source) = container_at(world, above) {
            let Some(hopper_ref) = hopper.container_ref() else {
                return false;
            };
            let from_hopper_time = world.get_block_entity(above).and_then(|block_entity| {
                block_entity
                    .downcast_ref::<HopperBlockEntity>()
                    .map(HopperBlockEntity::ticked_game_time)
            });
            // The destination target is measured before anything is locked: its
            // size is read through the container lock, so building it inside the
            // guard would lock this hopper twice.
            let into = TransferTarget::single(hopper_ref.clone());

            let mut refs = Vec::with_capacity(source.parts.len() + 1);
            refs.push(hopper_ref);
            refs.extend(source.parts.iter().cloned());
            let mut guard = ContainerLockGuard::lock_all(&refs);

            for slot in source.slots(&guard, Direction::Down) {
                if try_take_in_item_from_slot(
                    &mut guard,
                    &source,
                    hopper,
                    &into,
                    slot,
                    from_hopper_time,
                ) {
                    return true;
                }
            }

            // A container above never falls through to item entities.
            return false;
        }

        if blocks_hopper_pickup(world, above, state) {
            return false;
        }

        for entity in items_at_and_above(world, hopper) {
            if add_item_from_entity(hopper, entity.as_ref()) {
                return true;
            }
        }
        false
    }

    /// Vanilla `HopperBlockEntity.entityInside`: an item entity that overlaps the
    /// hopper's upper half moves immediately instead of waiting for the tick.
    pub(crate) fn entity_inside(
        &self,
        world: &Arc<World>,
        pos: BlockPos,
        state: BlockStateId,
        entity: &dyn Entity,
    ) {
        let Some(item_entity) = entity.downcast_ref::<ItemEntity>() else {
            return;
        };
        if item_entity.get_item().is_empty() || !self.suck_aabb().intersects(entity.bounding_box())
        {
            return;
        }
        self.try_move_items(world, pos, state, || add_item_from_entity(self, entity));
    }

    /// Vanilla `Hopper.getSuckAabb` moved to this hopper's world position.
    fn suck_aabb(&self) -> WorldAabb {
        let pos = self.get_block_pos();
        WorldAabb::new(
            f64::from(pos.x()),
            f64::from(pos.y()) + SUCK_MIN_Y,
            f64::from(pos.z()),
            f64::from(pos.x()) + 1.0,
            f64::from(pos.y()) + SUCK_MAX_Y,
            f64::from(pos.z()) + 1.0,
        )
    }
}

/// One logical container a hopper moves items through.
///
/// Vanilla backs a double chest with a `CompoundContainer` whose slot space is
/// the two halves concatenated. Vecta keeps every half as its own container, so
/// a logical slot index is mapped onto the half that stores it.
struct TransferTarget {
    /// The backing containers, in joined slot order.
    parts: Vec<ContainerRef>,
    /// The logical slot each part starts at.
    starts: Vec<usize>,
    /// Total slots across every part.
    size: usize,
    /// The hopper block entity backing this target, when it is a hopper.
    hopper: Option<SharedBlockEntity>,
}

impl TransferTarget {
    fn new(parts: Vec<ContainerRef>) -> Self {
        let mut starts = Vec::with_capacity(parts.len());
        let mut size = 0;
        for part in &parts {
            starts.push(size);
            size += container_size(part);
        }
        Self {
            parts,
            starts,
            size,
            hopper: None,
        }
    }

    fn single(part: ContainerRef) -> Self {
        Self::new(vec![part])
    }

    fn with_hopper(mut self, hopper: Option<SharedBlockEntity>) -> Self {
        self.hopper = hopper;
        self
    }

    /// The hopper behind this target, when it is one.
    fn hopper(&self) -> Option<&HopperBlockEntity> {
        self.hopper
            .as_ref()
            .and_then(|block_entity| block_entity.downcast_ref::<HopperBlockEntity>())
    }

    /// Maps a logical slot onto its backing container and local slot.
    fn locate(&self, slot: usize) -> Option<(usize, usize)> {
        let index = self.starts.iter().rposition(|start| *start <= slot)?;
        Some((index, slot - self.starts[index]))
    }

    fn id(&self, index: usize) -> ContainerId {
        self.parts[index].container_id()
    }

    /// Vanilla `HopperBlockEntity.getSlots`: a single worldly container exposes
    /// its face view, anything else (including a joined pair) is flat.
    fn slots(&self, guard: &ContainerLockGuard, direction: Direction) -> Vec<usize> {
        if let [only] = self.parts.as_slice()
            && let Some(face_slots) = guard
                .get(only.container_id())
                .and_then(|container| container.slots_for_face(direction))
        {
            return face_slots.to_vec();
        }
        (0..self.size).collect()
    }

    /// Vanilla `HopperBlockEntity.isFullContainer`.
    fn is_full(&self, guard: &ContainerLockGuard, direction: Direction) -> bool {
        self.slots(guard, direction).into_iter().all(|slot| {
            let Some((part, local)) = self.locate(slot) else {
                return true;
            };
            guard.get(self.id(part)).is_none_or(|container| {
                let stack = container.get_item(local);
                stack.count() >= stack.max_stack_size()
            })
        })
    }

    /// Vanilla `Container.isEmpty` over the whole logical container.
    fn is_empty(&self, guard: &ContainerLockGuard) -> bool {
        (0..self.size).all(|slot| {
            let Some((part, local)) = self.locate(slot) else {
                return true;
            };
            guard
                .get(self.id(part))
                .is_none_or(|container| container.get_item(local).is_empty())
        })
    }

    /// Vanilla `container.setChanged()`; a joined pair notifies both halves.
    fn mark_changed(&self, guard: &mut ContainerLockGuard) {
        for part in &self.parts {
            guard.set_changed(part.container_id());
        }
    }
}

fn container_size(container: &ContainerRef) -> usize {
    ContainerLockGuard::lock_all(slice::from_ref(container))
        .get(container.container_id())
        .map_or(0, Container::get_container_size)
}

/// Vanilla `HopperBlockEntity.getContainerAt` for a block position.
fn container_at(world: &Arc<World>, pos: BlockPos) -> Option<TransferTarget> {
    let state = world.get_block_state(pos);
    let block_entity = world.get_block_entity(pos)?;
    unpack_pending_loot(&block_entity);

    if block_entity.is::<ChestBlockEntity>() {
        // Vanilla `getBlockContainer` asks `ChestBlock.getContainer(..., true)`,
        // so a covered double chest still accepts items as one container.
        return match ChestBlock::combine(state, world.as_ref(), pos, true) {
            ChestPair::None => None,
            ChestPair::Single(only) => Some(TransferTarget::single(
                ContainerRef::from_block_entity(only)?,
            )),
            ChestPair::Double(first, second) => {
                let first = ContainerRef::from_block_entity(first)?;
                let second = ContainerRef::from_block_entity(second)?;
                Some(TransferTarget::new(vec![first, second]))
            }
        };
    }

    let is_hopper = block_entity.is::<HopperBlockEntity>();
    let container = ContainerRef::from_block_entity(Arc::clone(&block_entity))?;
    let target = TransferTarget::single(container);
    Some(if is_hopper {
        target.with_hopper(Some(block_entity))
    } else {
        target
    })
}

/// Vanilla `RandomizableContainerBlockEntity` rolls a pending loot table on the
/// first contents read; a hopper reads the target container directly, so the
/// roll happens here instead.
fn unpack_pending_loot(block_entity: &SharedBlockEntity) {
    if let Some(chest) = block_entity.downcast_ref::<ChestBlockEntity>() {
        chest.unpack_loot_table(None);
    } else if let Some(barrel) = block_entity.downcast_ref::<BarrelBlockEntity>() {
        barrel.unpack_loot_table(None);
    } else if let Some(shulker_box) = block_entity.downcast_ref::<ShulkerBoxBlockEntity>() {
        shulker_box.unpack_loot_table(None);
    } else if let Some(hopper) = block_entity.downcast_ref::<HopperBlockEntity>() {
        hopper.unpack_loot_table(None);
    }
}

/// Vanilla `HopperBlockEntity.addItem(from, container, itemStack, direction)`.
fn add_item(
    guard: &mut ContainerLockGuard,
    target: &TransferTarget,
    mut stack: ItemStack,
    direction: Option<Direction>,
    from_hopper_time: Option<i64>,
    target_hopper: Option<&HopperBlockEntity>,
) -> ItemStack {
    let slots: Vec<usize> = direction.map_or_else(
        || (0..target.size).collect(),
        |direction| target.slots(guard, direction),
    );

    for slot in slots {
        if stack.is_empty() {
            break;
        }
        stack = try_move_in_item(
            guard,
            target,
            stack,
            slot,
            direction,
            from_hopper_time,
            target_hopper,
        );
    }

    stack
}

/// Vanilla `HopperBlockEntity.tryMoveInItem`: moves as much of `stack` into one
/// slot as fits and returns what is left.
fn try_move_in_item(
    guard: &mut ContainerLockGuard,
    target: &TransferTarget,
    stack: ItemStack,
    slot: usize,
    direction: Option<Direction>,
    from_hopper_time: Option<i64>,
    target_hopper: Option<&HopperBlockEntity>,
) -> ItemStack {
    let Some((part, local)) = target.locate(slot) else {
        return stack;
    };
    let id = target.id(part);
    // A joined pair is not a `WorldlyContainer`, so only a lone container has a
    // face rule to honour.
    let worldly = target.parts.len() == 1;

    let (current, can_place, was_empty) = {
        let Some(container) = guard.get(id) else {
            return stack;
        };
        (
            container.get_item(local).clone(),
            can_place_item_in_container(container, &stack, local, direction, worldly),
            target.is_empty(guard),
        )
    };
    if !can_place {
        return stack;
    }

    let mut remaining = stack;
    let mut placed = ItemStack::empty();
    let mut success = false;
    if current.is_empty() {
        placed = remaining.clone();
        remaining = ItemStack::empty();
        success = !placed.is_empty();
    } else if can_merge_items(&current, &remaining) {
        let space = remaining.max_stack_size() - current.count();
        let count = remaining.count().min(space);
        if count > 0 {
            placed = current;
            placed.grow(count);
            remaining.shrink(count);
            success = true;
        }
    }

    if success {
        if let Some(container) = guard.get_mut(id) {
            container.set_item(local, placed);
        }

        if was_empty
            && let Some(hopper) = target_hopper
            && !hopper.is_on_custom_cooldown()
        {
            // A hopper that already ticked this game tick has effectively waited
            // one tick of the cooldown already.
            let skip = i32::from(
                from_hopper_time.is_some_and(|ticked| hopper.ticked_game_time() >= ticked),
            );
            hopper.set_cooldown(MOVE_ITEM_SPEED - skip);
        }

        target.mark_changed(guard);
    }

    remaining
}

/// Vanilla `HopperBlockEntity.tryTakeInItemFromSlot`.
///
/// `into` is the hopper's own target, measured by the caller before the guard
/// locks anything.
fn try_take_in_item_from_slot(
    guard: &mut ContainerLockGuard,
    source: &TransferTarget,
    hopper: &HopperBlockEntity,
    into: &TransferTarget,
    slot: usize,
    from_hopper_time: Option<i64>,
) -> bool {
    let Some((part, local)) = source.locate(slot) else {
        return false;
    };
    let id = source.id(part);
    let into_id = into.id(0);
    let worldly = source.parts.len() == 1;

    let (stack, can_take) = {
        let (Some(from), Some(destination)) = (guard.get(id), guard.get(into_id)) else {
            return false;
        };
        let stack = from.get_item(local).clone();
        let can_take = can_take_item_from_container(
            from,
            destination,
            &stack,
            local,
            Some(Direction::Down),
            worldly,
        );
        (stack, can_take)
    };
    if stack.is_empty() || !can_take {
        return false;
    }

    let original_count = stack.count();
    let removed = guard
        .get_mut(id)
        .map_or_else(ItemStack::empty, |container| {
            container.remove_item(local, 1)
        });
    if removed.is_empty() {
        return false;
    }

    let remaining = add_item(guard, into, removed, None, from_hopper_time, Some(hopper));
    if remaining.is_empty() {
        guard.set_changed(id);
        return true;
    }

    restore_source(guard, id, local, stack, original_count);
    false
}

/// Vanilla restores a source slot after a failed move: the live stack keeps its
/// original count, and a single item is written back through `setItem`, which
/// also notifies.
fn restore_source(
    guard: &mut ContainerLockGuard,
    id: ContainerId,
    slot: usize,
    stack: ItemStack,
    original_count: i32,
) {
    let mut restored = stack;
    restored.set_count(original_count);
    if original_count == 1 {
        guard.set_item(id, slot, restored);
    } else if let Some(container) = guard.get_mut(id) {
        container.set_item(slot, restored);
    }
}

/// Vanilla `HopperBlockEntity.addItem(Container, ItemEntity)`.
fn add_item_from_entity(hopper: &HopperBlockEntity, entity: &dyn Entity) -> bool {
    let Some(item_entity) = entity.downcast_ref::<ItemEntity>() else {
        return false;
    };
    let copy = item_entity.get_item();
    if copy.is_empty() {
        return false;
    }
    let Some(hopper_ref) = hopper.container_ref() else {
        return false;
    };

    let target = TransferTarget::single(hopper_ref);
    let mut guard = ContainerLockGuard::lock_all(&target.parts);
    let remaining = add_item(&mut guard, &target, copy, None, None, Some(hopper));
    if remaining.is_empty() {
        item_entity.set_item(ItemStack::empty());
        entity.set_removed(RemovalReason::Discarded);
        return true;
    }
    item_entity.set_item(remaining);
    false
}

/// Vanilla `HopperBlockEntity.getItemsAtAndAbove`.
fn items_at_and_above(world: &Arc<World>, hopper: &HopperBlockEntity) -> Vec<SharedEntity> {
    let aabb = hopper.suck_aabb();
    world.get_entities_in_aabb_matching(&aabb, |entity| {
        entity.is_alive() && entity.is::<ItemEntity>()
    })
}

/// Vanilla `HopperBlockEntity.suckInItems`'s block check: a full collision block
/// above a grid-aligned hopper stops item pickup unless it is tagged as
/// hopper-transparent.
fn blocks_hopper_pickup(world: &Arc<World>, pos: BlockPos, state: BlockStateId) -> bool {
    if state.is_air() {
        return false;
    }
    let shape = BLOCK_BEHAVIORS
        .get_behavior(state.get_block())
        .get_collision_shape(state, world.as_ref(), pos, BlockCollisionContext::empty());
    is_shape_full_block(shape) && !state.get_block().has_tag(&BlockTag::DOES_NOT_BLOCK_HOPPERS)
}

/// Vanilla `HopperBlockEntity.canPlaceItemInContainer`.
fn can_place_item_in_container(
    container: &dyn Container,
    stack: &ItemStack,
    slot: usize,
    direction: Option<Direction>,
    worldly: bool,
) -> bool {
    if !container.can_place_item(slot, stack) {
        return false;
    }
    match direction {
        Some(direction) if worldly => container.can_place_item_through_face(slot, stack, direction),
        _ => true,
    }
}

/// Vanilla `HopperBlockEntity.canTakeItemFromContainer`.
fn can_take_item_from_container(
    from: &dyn Container,
    into: &dyn Container,
    stack: &ItemStack,
    slot: usize,
    direction: Option<Direction>,
    worldly: bool,
) -> bool {
    from.can_take_item(into, slot, stack)
        && match direction {
            Some(direction) if worldly => from.can_take_item_through_face(slot, stack, direction),
            _ => true,
        }
}

/// Vanilla `HopperBlockEntity.canMergeItems`.
fn can_merge_items(current: &ItemStack, incoming: &ItemStack) -> bool {
    current.count() <= current.max_stack_size()
        && ItemStack::is_same_item_same_components(current, incoming)
}

impl BlockEntity for HopperBlockEntity {
    fn base(&self) -> &BlockEntityBase {
        &self.base
    }

    fn pre_remove_side_effects(&self, pos: BlockPos, _state: BlockStateId) {
        // Vanilla `Containers.dropContents` reads the slots, which unpacks the
        // loot table first.
        self.unpack_loot_table(None);
        let items = {
            let mut container = self.container.lock();
            mem::replace(&mut container.items, vec![ItemStack::empty(); HOPPER_SLOTS])
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
        let cooldown_time = nbt_view.int("TransferCooldown").unwrap_or(NO_COOLDOWN_TIME);
        if self.loot.load(nbt) {
            // Vanilla `RandomizableContainerBlockEntity.loadFromTag`: a stored
            // loot table replaces the item list, it does not accompany it.
            drop(container);
            self.set_cooldown(cooldown_time);
            return;
        }

        if let Some(items_list) = nbt_view.list("Items")
            && let Some(compounds) = items_list.compounds()
        {
            for compound in compounds {
                if let Some(slot) = compound.byte("Slot") {
                    let slot = slot as usize;
                    if slot < HOPPER_SLOTS
                        && let Some(item) = ItemStack::from_borrowed_compound(&compound)
                    {
                        container.items[slot] = item;
                    }
                }
            }
        }
        drop(container);
        self.set_cooldown(cooldown_time);
    }

    fn save_additional(&self, nbt: &mut NbtCompound) {
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
        // `saveAdditional` writes the cooldown outside the loot-table branch.
        nbt.insert("TransferCooldown", self.cooldown_time());
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
                "failed to collect the container component of the hopper at {:?}: {error}",
                self.get_block_pos()
            ),
        }
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

impl Container for HopperContainer {
    fn items(&self) -> &[ItemStack] {
        &self.items
    }

    fn items_mut(&mut self) -> &mut [ItemStack] {
        &mut self.items
    }

    fn get_container_size(&self) -> usize {
        HOPPER_SLOTS
    }

    fn set_item(&mut self, slot: usize, mut stack: ItemStack) {
        if slot < HOPPER_SLOTS {
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

    fn test_hopper() -> HopperBlockEntity {
        init_vanilla_registry();
        HopperBlockEntity::new(
            Weak::new(),
            BlockPos::new(1, 2, 3),
            vanilla_blocks::HOPPER.default_state(),
        )
    }

    #[test]
    fn set_item_limits_a_stack_to_the_item_maximum() {
        let hopper = test_hopper();
        hopper
            .container
            .lock()
            .set_item(0, ItemStack::with_count(&vanilla_items::STONE, 100));

        assert_eq!(hopper.container.lock().get_item(0).count(), 64);
    }

    #[test]
    fn pre_remove_clears_every_slot() {
        let hopper = test_hopper();
        hopper
            .container
            .lock()
            .set_item(0, ItemStack::new(&vanilla_items::STONE));

        hopper.pre_remove_side_effects(
            BlockPos::new(1, 2, 3),
            vanilla_blocks::HOPPER.default_state(),
        );

        let container = hopper.container.lock();
        assert_eq!(container.items.len(), HOPPER_SLOTS);
        assert!(container.items.iter().all(ItemStack::is_empty));
    }

    #[test]
    fn a_missing_cooldown_loads_as_no_cooldown() {
        let hopper = test_hopper();
        let nbt = NbtCompound::new();
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("empty NBT");
        hopper.load_additional(&borrowed);

        assert_eq!(hopper.cooldown_time(), NO_COOLDOWN_TIME);
        assert!(!hopper.is_on_cooldown());
    }

    #[test]
    fn the_transfer_cooldown_survives_an_nbt_round_trip() {
        let hopper = test_hopper();
        hopper.set_cooldown(5);
        hopper
            .container
            .lock()
            .set_item(2, ItemStack::with_count(&vanilla_items::DIAMOND, 3));

        let mut nbt = NbtCompound::new();
        hopper.save_additional(&mut nbt);
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("hopper NBT decodes");

        let loaded = test_hopper();
        loaded.load_additional(&borrowed);

        assert_eq!(loaded.cooldown_time(), 5);
        assert!(loaded.is_on_cooldown());
        assert_eq!(loaded.container.lock().get_item(2).count(), 3);
    }

    #[test]
    fn a_custom_name_survives_an_nbt_round_trip() {
        let hopper = test_hopper();
        let mut nbt = NbtCompound::new();
        nbt.insert(
            "CustomName",
            TextComponent::plain("Feed".to_owned()).to_codec_nbt(),
        );
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("identity NBT reads");
        hopper.load_additional(&borrowed);

        let mut saved = NbtCompound::new();
        hopper.save_additional(&mut saved);
        let mut bytes = Vec::new();
        saved.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("hopper NBT decodes");
        let loaded = test_hopper();
        loaded.load_additional(&borrowed);

        assert_eq!(
            loaded
                .identity()
                .display_name(|| TextComponent::plain("default")),
            TextComponent::plain("Feed"),
        );
    }
}
