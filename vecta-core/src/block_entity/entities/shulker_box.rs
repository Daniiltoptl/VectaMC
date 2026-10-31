use std::sync::{
    Arc, Weak,
    atomic::{AtomicI32, Ordering},
};

use glam::DVec3;
use simdnbt::{
    ToNbtTag,
    borrow::{BaseNbtCompound, NbtCompound as NbtCompoundView},
    owned::{NbtCompound, NbtList, NbtTag},
};
use vecta_registry::{
    ItemStackTemplate, REGISTRY,
    blocks::{
        behavior::PushReaction, block_state_ext::BlockStateExt, properties::BlockStateProperties,
    },
    data_components::{
        DataComponentMap, DataComponentPatch, ItemContainerContents, vanilla_components::CONTAINER,
    },
    item_stack::ItemStack,
    vanilla_block_entity_types,
};
use vecta_utils::{
    BlockLocalAabb, BlockPos, BlockStateId, Direction, DowncastType, DowncastTypeKey, Identifier,
    WorldAabb,
    geometry::{Aabb, Space},
    locks::SyncMutex,
    types::UpdateFlags,
};

use crate::{
    behavior::blocks::ShulkerBoxBlock,
    block_entity::{
        BlockEntity, BlockEntityBase, container_identity::ContainerIdentity,
        container_loot::ContainerLoot,
    },
    inventory::{
        container::Container,
        lock::{ContainerRef, SharedContainer},
    },
    physics::{CollisionWorld as _, MoverType, WorldCollisionProvider},
    player::Player,
    world::World,
};

/// Number of slots in a shulker box (3 rows of 9).
pub const SHULKER_BOX_SLOTS: usize = 27;
const ANIMATION_STEPS: u8 = 10;

/// The current animation state of a shulker box.
#[derive(Debug, Clone, Copy)]
pub enum AnimationStatus {
    /// Fully closed.
    Closed,
    /// Opening.
    Opening,
    /// Fully open.
    Opened,
    /// Closing.
    Closing,
}

pub struct ShulkerBoxAnimation {
    animation_status: AnimationStatus,
    progress: u8,
    old_progress: u8,
}

impl ShulkerBoxAnimation {
    fn progress(&self) -> f32 {
        f32::from(self.progress) / f32::from(ANIMATION_STEPS)
    }

    fn old_progress(&self) -> f32 {
        f32::from(self.old_progress) / f32::from(ANIMATION_STEPS)
    }
}

/// Block entity backing every shulker box color, including the undyed one.
pub struct ShulkerBoxBlockEntity {
    base: Arc<BlockEntityBase>,
    container: Arc<SyncMutex<ShulkerBoxContainer>>,
    container_ref: ContainerRef,
    animation: SyncMutex<ShulkerBoxAnimation>,
    open_count: AtomicI32,
    identity: ContainerIdentity,
    loot: ContainerLoot,
}

struct ShulkerBoxContainer {
    items: Vec<ItemStack>,
}

// SAFETY: This key is owned by Vecta and uniquely identifies `ShulkerBoxEntity`.
unsafe impl DowncastType for ShulkerBoxBlockEntity {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:block_entity/shulker_box");
}

// SAFETY: This key is owned by Vecta and uniquely identifies the independently
// lockable inventory data used by a shulker box block entity.
unsafe impl DowncastType for ShulkerBoxContainer {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:container/shulker_box");
}

fn do_neighbor_updates(world: &Arc<World>, pos: BlockPos, state: BlockStateId) {
    world.update_neighbour_shapes(state, pos, UpdateFlags::UPDATE_ALL, 512);
    world.update_neighbors_at(pos, state.get_block());
}

impl ShulkerBoxBlockEntity {
    /// Creates a new shulker box block entity.
    #[must_use]
    pub fn new(level: Weak<World>, pos: BlockPos, state: BlockStateId) -> Self {
        let base = Arc::new(BlockEntityBase::new(
            &vanilla_block_entity_types::SHULKER_BOX,
            level,
            pos,
            state,
        ));
        let container = Arc::new(SyncMutex::new(ShulkerBoxContainer {
            items: vec![ItemStack::empty(); SHULKER_BOX_SLOTS],
        }));
        let shared_container: SharedContainer =
            Arc::<SyncMutex<ShulkerBoxContainer>>::clone(&container);
        Self {
            container_ref: ContainerRef::owned_by_block_entity(shared_container, Arc::clone(&base)),
            base,
            container,
            animation: SyncMutex::new(ShulkerBoxAnimation {
                animation_status: AnimationStatus::Closed,
                progress: 0,
                old_progress: 0,
            }),
            open_count: AtomicI32::new(0),
            identity: ContainerIdentity::new(),
            loot: ContainerLoot::new(),
        }
    }

    /// Returns the custom name and lock of this shulker box.
    #[must_use]
    pub const fn identity(&self) -> &ContainerIdentity {
        &self.identity
    }

    /// Rolls this box's pending loot table, if any.
    pub fn unpack_loot_table(&self, player: Option<&Player>) {
        self.loot.unpack(
            self.get_block_pos(),
            &mut self.container.lock().items,
            player,
        );
    }

    fn update_animation(&self, world: &Arc<World>, pos: BlockPos, state: BlockStateId) {
        let mut animation = self.animation.lock();
        animation.old_progress = animation.progress;
        match animation.animation_status {
            AnimationStatus::Closed => animation.progress = 0,
            AnimationStatus::Opening => {
                animation.progress += 1;
                if animation.old_progress == 0 {
                    do_neighbor_updates(world, pos, state);
                }

                if animation.progress >= ANIMATION_STEPS {
                    animation.animation_status = AnimationStatus::Opened;
                    animation.progress = ANIMATION_STEPS;
                    do_neighbor_updates(world, pos, state);
                }

                drop(animation);
                self.move_collided_entities(world, pos, state);
            }
            AnimationStatus::Opened => animation.progress = ANIMATION_STEPS,
            AnimationStatus::Closing => {
                animation.progress = animation.progress.saturating_sub(1);
                if animation.old_progress == ANIMATION_STEPS {
                    do_neighbor_updates(world, pos, state);
                }

                if animation.progress == 0 {
                    animation.animation_status = AnimationStatus::Closed;
                    animation.progress = 0;
                    do_neighbor_updates(world, pos, state);
                }
            }
        }
    }

    fn move_collided_entities(&self, world: &Arc<World>, pos: BlockPos, state: BlockStateId) {
        let direction = state.get_value(&BlockStateProperties::FACING);

        let aabb: WorldAabb = {
            let animation = self.animation.lock();
            Self::get_progress_delta_aabb(
                1.0,
                direction,
                animation.old_progress(),
                animation.progress(),
                DVec3::from(pos.get_bottom_center()),
            )
        };

        let entities = world.get_entities_in_aabb(&aabb);
        for entity in entities {
            if entity.piston_push_reaction() == PushReaction::IgnoreEntity {
                continue;
            }
            let (offset_x, offset_y, offset_z) = direction.offset();
            entity.move_entity(
                MoverType::ShulkerBox,
                DVec3::new(
                    (aabb.width() + 0.01) * f64::from(offset_x),
                    (aabb.height() + 0.01) * f64::from(offset_y),
                    (aabb.depth() + 0.01) * f64::from(offset_z),
                ),
            );
        }
    }

    /// Builds the item form of a placed, possibly-filled shulker box.
    /// Vanilla `ShulkerBoxBlock.playerWillDestroy` applies
    /// `blockEntity.collectComponents()`, so the name, the lock and the
    /// contents all travel with the dropped box.
    pub fn shulker_box_as_item(&self, state: BlockStateId) -> ItemStack {
        let block_item = REGISTRY.items.by_block(state.get_block());

        let mut components = DataComponentMap::new();
        self.collect_implicit_components(&mut components);
        let mut patch = DataComponentPatch::new();
        for (key, data) in components.iter() {
            patch.set_raw(key.clone(), data.clone());
        }

        ItemStack::with_count_and_patch(block_item, 1, patch)
    }

    /// Calculates the bounding box of a shulker during its opening animation.
    ///
    /// The bounds are expanded in the opening direction based on the animation progress.
    #[must_use]
    pub fn get_progress_delta_aabb<I: Space>(
        size: f32,
        direction: Direction,
        progress_from: f32,
        progress_to: f32,
        position: DVec3,
    ) -> Aabb<DVec3, I> {
        let size = f64::from(size);
        let bounds =
            Aabb::<DVec3, I>::new(-size * 0.5, 0.0, -size * 0.5, size * 0.5, size, size * 0.5);

        let max_movement = f64::from(progress_from.max(progress_to));
        let min_movement = f64::from(progress_from.min(progress_to));

        let dir = DVec3::from(direction.offset_vec());

        bounds
            .expand_towards(dir * max_movement * size)
            .contract(-dir * (1.0 + min_movement) * size)
            .translate(position)
    }

    /// Block-local bounds of the box including the lid at its current progress.
    #[must_use]
    pub fn get_bounding_box(&self, state: BlockStateId) -> BlockLocalAabb {
        let bottom_center = DVec3::new(0.5, 0.0, 0.5);
        Self::get_progress_delta_aabb(
            1.0,
            state.get_value(ShulkerBoxBlock::FACING),
            -1.0,
            0.5 * self.progress(1.0),
            bottom_center,
        )
    }

    /// Returns whether the shulker box can open
    #[must_use]
    pub fn can_open(&self, state: BlockStateId, world: &Arc<World>, pos: BlockPos) -> bool {
        if !matches!(self.animation_status(), AnimationStatus::Closed) {
            return true;
        }

        let direction = state.get_value(ShulkerBoxBlock::FACING);

        let lid_open_bounding_box: WorldAabb = Self::get_progress_delta_aabb(
            1.0,
            direction,
            0.0,
            0.5,
            DVec3::from(pos.get_bottom_center()),
        )
        .deflate(1.0E-6);

        let collision = WorldCollisionProvider::new(world);
        !collision.has_block_collision(&lid_open_bounding_box)
    }

    /// Checks if the block entity's container has any items.
    ///
    /// Vanilla `RandomizableContainerBlockEntity.isEmpty` unpacks the pending
    /// loot table first, so a loot-backed box is never reported empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.unpack_loot_table(None);
        self.container.lock().is_empty()
    }

    /// Collect all the items inside the shulker box into an `ItemContainerContents`
    ///
    /// # Panics
    /// Panics if shulker box somehow has more than 256 slots. This should never happen
    #[must_use]
    pub fn collect_components(&self) -> ItemContainerContents {
        let container = self.container.lock();

        let size = container.get_container_size();
        let slots: Vec<Option<ItemStackTemplate>> = (0..size)
            .map(|i| {
                let item = container.get_item(i);
                if item.is_empty() {
                    None
                } else {
                    ItemStackTemplate::from_stack(item).ok()
                }
            })
            .collect();

        ItemContainerContents::new(slots)
            .expect("shulker box slot count is always within the 256 container-contents limit")
    }

    /// Interpolated progress for smooth rendering between ticks.
    #[must_use]
    pub fn progress(&self, partial_tick: f32) -> f32 {
        let animation = self.animation.lock();
        let new = animation.progress();
        let old = animation.old_progress();
        old + partial_tick * (new - old)
    }

    /// Get the current animation state
    #[must_use]
    pub fn animation_status(&self) -> AnimationStatus {
        self.animation.lock().animation_status
    }
}

impl BlockEntity for ShulkerBoxBlockEntity {
    fn base(&self) -> &BlockEntityBase {
        &self.base
    }

    fn tick(&self, world: &Arc<World>) {
        self.update_animation(world, self.base.pos, self.base.block_state());
    }

    fn trigger_event(&self, kind: i32, data: i32) -> bool {
        if kind == 1 {
            self.open_count.store(data, Ordering::Relaxed);
            if data == 0 {
                self.animation.lock().animation_status = AnimationStatus::Closing;
            }

            if data == 1 {
                self.animation.lock().animation_status = AnimationStatus::Opening;
            }

            true
        } else {
            false
        }
    }

    fn load_additional(&self, nbt: &BaseNbtCompound<'_>) {
        let nbt_view: NbtCompoundView<'_, '_> = nbt.into();
        self.identity.load(nbt);
        let mut container = self.container.lock();
        container.items.fill(ItemStack::empty());
        if self.loot.load(nbt) {
            // Vanilla `RandomizableContainerBlockEntity.loadFromTag`: a stored
            // loot table replaces the item list, it does not accompany it.
            return;
        }

        let Some(items_list) = nbt_view.list("Items") else {
            return;
        };
        let Some(compounds) = items_list.compounds() else {
            return;
        };

        for compound in compounds {
            let Some(slot) = compound.byte("Slot") else {
                continue;
            };
            let slot = slot as usize;
            if slot >= SHULKER_BOX_SLOTS {
                continue;
            }
            if let Some(item) = ItemStack::from_borrowed_compound(&compound) {
                container.items[slot] = item;
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
        for (slot, item) in container.items().iter().enumerate() {
            if item.is_empty() {
                continue;
            }
            let NbtTag::Compound(mut item_nbt) = item.clone().to_nbt_tag() else {
                continue;
            };
            item_nbt.insert("Slot", slot as i8);
            items.push(item_nbt);
        }
        nbt.insert("Items", NbtList::Compound(items));
    }

    fn apply_implicit_components(&self, item: &ItemStack) -> Vec<Identifier> {
        let mut consumed = self.identity.apply_implicit_components(item);
        if let Some(key) = self.loot.apply_implicit_components(item) {
            consumed.push(key);
        }

        if let Some(contents) = item.get(CONTAINER) {
            let mut container = self.container.lock();
            container.items.fill(ItemStack::empty());
            for (slot, template) in contents.items().iter().enumerate() {
                if slot >= SHULKER_BOX_SLOTS {
                    break;
                }
                if let Some(template) = template {
                    container.items_mut()[slot] = ItemStack::with_count_and_patch(
                        template.item(),
                        template.count(),
                        template.components().clone(),
                    );
                }
            }
            consumed.push(CONTAINER.key().clone());
        }

        consumed
    }

    fn collect_implicit_components(&self, components: &mut DataComponentMap) {
        self.identity.collect_implicit_components(components);
        self.loot.collect_implicit_components(components);
        components.set(CONTAINER, Some(self.collect_components()));
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

impl Container for ShulkerBoxContainer {
    fn items(&self) -> &[ItemStack] {
        &self.items
    }

    fn items_mut(&mut self) -> &mut [ItemStack] {
        &mut self.items
    }

    fn get_container_size(&self) -> usize {
        SHULKER_BOX_SLOTS
    }

    fn set_item(&mut self, slot: usize, mut stack: ItemStack) {
        if slot < SHULKER_BOX_SLOTS {
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
    use vecta_registry::data_components::vanilla_components::{CUSTOM_NAME, LOCK};
    use vecta_registry::{init_vanilla_registry, vanilla_blocks, vanilla_items};

    use super::*;

    fn test_box() -> (ShulkerBoxBlockEntity, BlockStateId) {
        init_vanilla_registry();
        let state = vanilla_blocks::SHULKER_BOX.default_state();
        (
            ShulkerBoxBlockEntity::new(Weak::new(), BlockPos::new(1, 2, 3), state),
            state,
        )
    }

    /// Loads `CustomName`/`lock` the way `/data merge` or a placed block item
    /// would.
    fn load_identity(
        shulker_box: &ShulkerBoxBlockEntity,
        name: Option<&str>,
        lock_item: Option<&str>,
    ) {
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
        shulker_box.load_additional(&borrowed);
    }

    #[test]
    fn the_custom_name_and_lock_survive_an_nbt_round_trip() {
        let (shulker_box, state) = test_box();
        load_identity(&shulker_box, Some("Loot"), Some("minecraft:diamond"));

        let mut nbt = NbtCompound::new();
        shulker_box.save_additional(&mut nbt);
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed = read_borrowed_compound(&mut Cursor::new(bytes.as_slice()))
            .expect("shulker NBT decodes");

        let loaded = ShulkerBoxBlockEntity::new(Weak::new(), BlockPos::new(1, 2, 3), state);
        loaded.load_additional(&borrowed);

        assert!(loaded.identity().is_locked());
        assert_eq!(
            loaded
                .identity()
                .display_name(|| TextComponent::plain("default")),
            TextComponent::plain("Loot"),
        );
    }

    #[test]
    fn the_dropped_box_keeps_its_name_lock_and_contents() {
        let (shulker_box, state) = test_box();
        load_identity(&shulker_box, Some("Loot"), Some("minecraft:diamond"));
        shulker_box
            .container
            .lock()
            .set_item(3, ItemStack::with_count(&vanilla_items::DIAMOND, 2));

        let item = shulker_box.shulker_box_as_item(state);

        assert!(item.is(&vanilla_items::SHULKER_BOX));
        assert_eq!(
            item.get(CUSTOM_NAME).cloned(),
            Some(TextComponent::plain("Loot")),
        );
        assert!(item.get(LOCK).is_some(), "the lock travels with the box");
        let contents = item
            .get(CONTAINER)
            .expect("the dropped box carries its contents");
        let stack = contents
            .items()
            .get(3)
            .and_then(Option::as_ref)
            .expect("slot 3 holds the diamonds")
            .create();
        assert!(stack.is(&vanilla_items::DIAMOND));
        assert_eq!(stack.count(), 2);
    }
}
