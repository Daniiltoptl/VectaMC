//! Vanilla `ItemFrame` entity: displayed item, rotation, support checks and drops.

use std::sync::{Arc, Weak};

use glam::DVec3;
use simdnbt::borrow::NbtCompound as BorrowedNbtCompoundView;
use simdnbt::owned::NbtCompound;
use vecta_macros::entity_behavior;
use vecta_math::DEGREE_90;
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::data_components::vanilla_components::{CUSTOM_NAME, MAP_ID};
use vecta_registry::entity_type::EntityTypeRef;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::sound_event::SoundEventRef;
use vecta_registry::vanilla_entity_data::ItemFrameEntityData;
use vecta_registry::vanilla_game_rules::ENTITY_DROPS;
use vecta_registry::{
    sound_events, vanilla_blocks, vanilla_damage_type_tags, vanilla_entities, vanilla_game_events,
    vanilla_items,
};
use vecta_utils::locks::SyncMutex;
use vecta_utils::nbt::NbtNumeric as _;
use vecta_utils::types::InteractionHand;
use vecta_utils::{BlockPos, Direction, DowncastType, DowncastTypeKey, WorldAabb, axis::Axis};

use crate::behavior::{BLOCK_BEHAVIORS, BlockCollisionContext, InteractionResult};
use crate::entity::block_attached_entity::{BlockAttachedEntity, BlockAttachedEntityBase};
use crate::entity::damage::DamageSource;
use crate::entity::{
    Entity, EntityBase, EntityBaseLoad, EntityBaseState, EntityEventSource as _, EntityMoveError,
    EntitySyncedData, ItemFrame, SharedEntity,
};
use crate::physics::{CollisionWorld as _, MoveResult, MoverType, WorldCollisionProvider};
use crate::player::Player;
use crate::world::World;

const NUM_ROTATIONS: i32 = 8;
const FRAME_SIZE: f64 = 0.75;
const FRAMED_MAP_SIZE: f64 = 1.0;
const FRAME_DEPTH: f64 = 0.0625;
const DROP_OFFSET: f32 = 0.15;

/// Persistent state that Vanilla keeps as plain `ItemFrame` fields.
#[derive(Debug, Clone, Copy)]
struct ItemFrameState {
    drop_chance: f32,
    fixed: bool,
}

impl ItemFrameState {
    const fn new() -> Self {
        Self {
            drop_chance: 1.0,
            fixed: false,
        }
    }
}

/// Item frame entity covering Vanilla `ItemFrame` and its `GlowItemFrame` subtype.
#[entity_behavior(class = "ItemFrame")]
pub struct ItemFrameEntity {
    base: EntityBase,
    entity_type: EntityTypeRef,
    entity_data: SyncMutex<ItemFrameEntityData>,
    block_attached_entity_base: BlockAttachedEntityBase,
    state: SyncMutex<ItemFrameState>,
}

// SAFETY: This key is owned by Vecta and uniquely identifies `ItemFrameEntity`.
unsafe impl DowncastType for ItemFrameEntity {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:entity/item_frame");
}

impl ItemFrameEntity {
    /// Creates a fresh item frame from the generic entity factory path.
    #[must_use]
    pub fn new(entity_type: EntityTypeRef, id: i32, position: DVec3, world: Weak<World>) -> Self {
        Self::new_attached(
            entity_type,
            id,
            BlockPos::new(
                position.x.floor() as i32,
                position.y.floor() as i32,
                position.z.floor() as i32,
            ),
            Direction::South,
            world,
        )
    }

    /// Creates a fresh item frame attached to `block_pos` and facing `direction`.
    #[must_use]
    pub fn new_attached(
        entity_type: EntityTypeRef,
        id: i32,
        block_pos: BlockPos,
        direction: Direction,
        world: Weak<World>,
    ) -> Self {
        let entity = Self {
            base: EntityBase::new_with_state(
                id,
                EntityBaseState::new_with_bounding_box(
                    Self::frame_center(block_pos, direction),
                    entity_type.dimensions,
                    Self::frame_bounding_box(block_pos, direction, false),
                )
                .with_rotation(Self::rotation_for_direction(direction)),
                world,
            ),
            entity_type,
            entity_data: SyncMutex::new(ItemFrameEntityData::new()),
            block_attached_entity_base: BlockAttachedEntityBase::new(block_pos),
            state: SyncMutex::new(ItemFrameState::new()),
        };
        entity
            .entity_data
            .lock()
            .hanging_entity
            .direction
            .set(direction);
        entity
    }

    /// Creates an item frame from persistent entity data.
    #[must_use]
    pub fn from_saved(entity_type: EntityTypeRef, load: EntityBaseLoad) -> Self {
        let position = load.position;
        Self {
            base: EntityBase::from_load(load, entity_type.dimensions),
            entity_type,
            entity_data: SyncMutex::new(ItemFrameEntityData::new()),
            block_attached_entity_base: BlockAttachedEntityBase::new(BlockPos::new(
                position.x.floor() as i32,
                position.y.floor() as i32,
                position.z.floor() as i32,
            )),
            state: SyncMutex::new(ItemFrameState::new()),
        }
    }

    /// Returns the framed item.
    #[must_use]
    pub fn item(&self) -> ItemStack {
        self.entity_data.lock().item.get().clone()
    }

    /// Returns the current item rotation in eighths of a turn.
    #[must_use]
    pub fn rotation(&self) -> i32 {
        *self.entity_data.lock().rotation.get()
    }

    /// Returns whether this frame ignores interactions, damage, movement and support loss.
    #[must_use]
    pub fn is_fixed(&self) -> bool {
        self.state.lock().fixed
    }

    fn is_glow_frame(&self) -> bool {
        self.entity_type == &vanilla_entities::GLOW_ITEM_FRAME
    }

    fn add_item_sound(&self) -> SoundEventRef {
        if self.is_glow_frame() {
            &sound_events::ENTITY_GLOW_ITEM_FRAME_ADD_ITEM
        } else {
            &sound_events::ENTITY_ITEM_FRAME_ADD_ITEM
        }
    }

    fn remove_item_sound(&self) -> SoundEventRef {
        if self.is_glow_frame() {
            &sound_events::ENTITY_GLOW_ITEM_FRAME_REMOVE_ITEM
        } else {
            &sound_events::ENTITY_ITEM_FRAME_REMOVE_ITEM
        }
    }

    fn rotate_item_sound(&self) -> SoundEventRef {
        if self.is_glow_frame() {
            &sound_events::ENTITY_GLOW_ITEM_FRAME_ROTATE_ITEM
        } else {
            &sound_events::ENTITY_ITEM_FRAME_ROTATE_ITEM
        }
    }

    fn break_sound(&self) -> SoundEventRef {
        if self.is_glow_frame() {
            &sound_events::ENTITY_GLOW_ITEM_FRAME_BREAK
        } else {
            &sound_events::ENTITY_ITEM_FRAME_BREAK
        }
    }

    /// Plays the placement sound of this frame variant.
    pub fn play_placement_sound(&self) {
        let sound = if self.is_glow_frame() {
            &sound_events::ENTITY_GLOW_ITEM_FRAME_PLACE
        } else {
            &sound_events::ENTITY_ITEM_FRAME_PLACE
        };
        self.play_sound(sound, 1.0, 1.0);
    }

    /// Returns the frame item matching this entity's variant.
    #[must_use]
    pub fn frame_item_stack(&self) -> ItemStack {
        let mut stack = if self.is_glow_frame() {
            ItemStack::new(&vanilla_items::GLOW_ITEM_FRAME)
        } else {
            ItemStack::new(&vanilla_items::ITEM_FRAME)
        };
        if let Some(name) = self.custom_name() {
            stack.set(CUSTOM_NAME, name);
        }
        stack
    }

    /// Sets the framed item, matching Vanilla by storing a single item.
    pub fn set_item(&self, item: ItemStack) {
        self.set_item_with_update(item, true);
    }

    /// Sets the framed item and optionally notifies nearby comparators.
    pub(crate) fn set_item_with_update(&self, mut item: ItemStack, update_comparators: bool) {
        if !item.is_empty() {
            item.set_count(1);
        }
        let has_item = !item.is_empty();
        self.entity_data.lock().item.set(item);
        self.recalculate_position_or_warn();
        if has_item {
            // Vanilla `ItemFrame.setItem` also plays this sound while loading saved
            // data, because `readAdditionalSaveData` reuses the same path.
            self.play_sound(self.add_item_sound(), 1.0, 1.0);
        }
        if update_comparators {
            self.update_output_signal_neighbours();
        }
    }

    fn set_rotation(&self, rotation: i32, update_comparators: bool) {
        self.entity_data
            .lock()
            .rotation
            .set(rotation % NUM_ROTATIONS);
        if update_comparators {
            self.update_output_signal_neighbours();
        }
    }

    fn update_output_signal_neighbours(&self) {
        if let Some(world) = self.level() {
            world.update_neighbor_for_output_signal(
                self.block_attached_entity_base.pos(),
                &vanilla_blocks::AIR,
            );
        }
    }

    fn set_direction(&self, direction: Direction) {
        self.entity_data
            .lock()
            .hanging_entity
            .direction
            .set(direction);
        self.base
            .set_rotation(Self::rotation_for_direction(direction));
        self.recalculate_position_or_warn();
    }

    fn try_recalculate_position(&self) -> Result<(), EntityMoveError> {
        let block_pos = self.block_attached_entity_base.pos();
        let direction = *self.entity_data.lock().hanging_entity.direction.get();
        let position = Self::frame_center(block_pos, direction);
        self.base.try_set_position(position)?;
        self.base.set_bounding_box(Self::frame_bounding_box(
            block_pos,
            direction,
            self.has_framed_map(),
        ));

        Ok(())
    }

    fn recalculate_position_or_warn(&self) {
        if let Err(error) = self.try_recalculate_position() {
            log::warn!(
                "failed to commit item frame {} position recalculation: {error}",
                self.base.id()
            );
        }
    }

    fn has_framed_map(&self) -> bool {
        self.entity_data.lock().item.get().has(MAP_ID)
    }

    /// Returns the collision box used for placement checks and overlapping frames.
    fn pop_box(&self) -> WorldAabb {
        Self::frame_bounding_box(
            self.block_attached_entity_base.pos(),
            self.direction(),
            false,
        )
    }

    fn has_level_collision(&self, pop_box: WorldAabb) -> bool {
        let Some(world) = self.level() else {
            return false;
        };
        let collision = WorldCollisionProvider::for_entity(&world, self);
        let context = BlockCollisionContext::entity(self.position().y, self.is_descending())
            .with_fall_distance(self.fall_distance());
        collision.has_block_collision_with_context(&pop_box, context)
            || collision.has_world_border_collision(&pop_box)
    }

    /// Returns whether another hanging entity occupies the same cell or facing.
    ///
    /// Vanilla compares against every `HangingEntity`. Paintings are not
    /// implemented yet, so item frames are the only hanging entities to check.
    fn can_coexist(&self, allow_intersecting_same_type: bool) -> bool {
        let Some(world) = self.level() else {
            return true;
        };
        let pop_box = self.pop_box();
        let direction = self.direction();
        !world.has_entity_in_aabb_matching(&pop_box, |other| {
            if other.id() == self.id() {
                return false;
            }
            let Some(other_frame) = other.as_item_frame() else {
                return false;
            };
            let intersects_same_type =
                !allow_intersecting_same_type && other.entity_type() == self.entity_type();
            intersects_same_type || other_frame.direction() == direction
        })
    }

    /// Returns whether the frame's backing block still supports it.
    fn survives(&self) -> bool {
        if self.is_fixed() {
            return true;
        }
        if self.has_level_collision(self.pop_box()) {
            return false;
        }
        let Some(world) = self.level() else {
            return false;
        };
        let direction = self.direction();
        let behind = self
            .block_attached_entity_base
            .pos()
            .relative(direction.opposite());
        let state = world.get_block_state(behind);
        let supported = state.is_solid()
            || (direction.axis().is_horizontal()
                && BLOCK_BEHAVIORS.get_behavior(state.get_block()).is_diode());
        supported && self.can_coexist(true)
    }

    pub(super) fn move_frame(&self, mover_type: MoverType, delta: DVec3) -> Option<MoveResult> {
        if self.is_fixed() {
            return None;
        }
        self.move_entity_block_attached_entity(mover_type, delta)
    }

    fn should_damage_drop_item(&self, source: &DamageSource) -> bool {
        !source.is(&vanilla_damage_type_tags::DamageTypeTag::IS_EXPLOSION)
            && !self.item().is_empty()
    }

    fn spawn_at_location(&self, item: ItemStack) {
        let Some(world) = self.level() else {
            return;
        };
        let step = self.direction().offset_vec().as_dvec3();
        let offset = f64::from(DROP_OFFSET);
        let position = self.position() + DVec3::new(step.x * offset, 0.0, step.z * offset);
        if let Some(entity) = world.spawn_item(position, item) {
            entity.set_default_pickup_delay();
        }
    }

    /// Drops the frame and its contents, matching Vanilla `ItemFrame.dropItem`.
    fn drop_frame_and_contents(&self, caused_by: Option<&dyn Entity>, with_frame: bool) {
        if self.is_fixed() {
            return;
        }
        let Some(world) = self.level() else {
            return;
        };

        let item = self.item();
        self.set_item_with_update(ItemStack::empty(), true);

        if !world.get_game_rule(&ENTITY_DROPS) {
            return;
        }
        if caused_by
            .and_then(|entity| entity.as_player())
            .is_some_and(Player::has_infinite_materials)
        {
            return;
        }

        if with_frame {
            self.spawn_at_location(self.frame_item_stack());
        }
        if item.is_empty() {
            return;
        }
        let drop_chance = self.state.lock().drop_chance;
        if rand::random_range(0.0_f32..1.0) < drop_chance {
            self.spawn_at_location(item);
        }
    }

    fn frame_center(block_pos: BlockPos, direction: Direction) -> DVec3 {
        let off = direction.offset_vec().as_dvec3() * 0.46875;
        block_pos.0.as_dvec3() + DVec3::splat(0.5) - off
    }

    fn rotation_for_direction(direction: Direction) -> (f32, f32) {
        if direction.is_horizontal() {
            (
                f32::from(direction_2d_data_value(direction)) * DEGREE_90,
                0.0,
            )
        } else {
            let pitch = match direction {
                Direction::Up => -DEGREE_90,
                Direction::Down => DEGREE_90,
                Direction::North | Direction::South | Direction::West | Direction::East => 0.0,
            };
            (0.0, pitch)
        }
    }

    fn frame_bounding_box(
        block_pos: BlockPos,
        direction: Direction,
        has_framed_map: bool,
    ) -> WorldAabb {
        let center = Self::frame_center(block_pos, direction);
        let size = if has_framed_map {
            FRAMED_MAP_SIZE
        } else {
            FRAME_SIZE
        };
        let x_size = if direction.axis() == Axis::X {
            FRAME_DEPTH
        } else {
            size
        };
        let y_size = if direction.axis() == Axis::Y {
            FRAME_DEPTH
        } else {
            size
        };
        let z_size = if direction.axis() == Axis::Z {
            FRAME_DEPTH
        } else {
            size
        };
        WorldAabb::new(
            center.x - x_size / 2.0,
            center.y - y_size / 2.0,
            center.z - z_size / 2.0,
            center.x + x_size / 2.0,
            center.y + y_size / 2.0,
            center.z + z_size / 2.0,
        )
    }
}

impl ItemFrame for ItemFrameEntity {
    fn direction(&self) -> Direction {
        *self.entity_data.lock().hanging_entity.direction.get()
    }

    fn analog_output(&self) -> i32 {
        let entity_data = self.entity_data.lock();
        if entity_data.item.get().is_empty() {
            0
        } else {
            *entity_data.rotation.get() % NUM_ROTATIONS + 1
        }
    }

    fn play_placement_sound(&self) {
        Self::play_placement_sound(self);
    }
}

impl Entity for ItemFrameEntity {
    fn base(&self) -> &EntityBase {
        &self.base
    }

    fn entity_type(&self) -> EntityTypeRef {
        self.entity_type
    }

    fn spawn_data(&self) -> i32 {
        self.direction().get_3d_data_value()
    }

    fn spawn_position(&self) -> DVec3 {
        let block_pos = self.block_attached_entity_base.pos();
        DVec3::new(
            f64::from(block_pos.x()),
            f64::from(block_pos.y()),
            f64::from(block_pos.z()),
        )
    }

    fn synced_data(&self) -> Option<&dyn EntitySyncedData> {
        Some(&self.entity_data)
    }

    fn tick(self: Arc<Self>) {
        self.tick_block_attached_entity();
    }

    fn is_pickable(&self) -> bool {
        self.is_pickable_block_attached_entity()
    }

    fn hurt(&self, world: &World, source: &DamageSource, amount: f32) -> bool {
        if !self.is_fixed() {
            if self.is_invulnerable_to_base(source) {
                return false;
            }
            if self.should_damage_drop_item(source) {
                let caused_by = source.causing_entity();
                self.drop_frame_and_contents(caused_by.as_ref().map(AsRef::as_ref), false);
                self.game_event_with_source_entity(
                    &vanilla_game_events::BLOCK_CHANGE,
                    caused_by.as_ref().map(AsRef::as_ref),
                );
                self.play_sound(self.remove_item_sound(), 1.0, 1.0);
                return true;
            }
            return self.hurt_block_attached_entity(world, source, amount);
        }

        let can_hurt_when_fixed = source.bypasses_invulnerability() || source.is_creative_player();
        can_hurt_when_fixed && self.hurt_block_attached_entity(world, source, amount)
    }

    fn skip_attack_interaction(&self, source: &SharedEntity) -> bool {
        self.skip_attack_interaction_block_attached_entity(source)
    }

    fn push_impulse(&self, impulse: DVec3) {
        if !self.is_fixed() {
            self.push_impulse_block_attached_entity(impulse);
        }
    }

    fn move_entity(self: Arc<Self>, mover_type: MoverType, delta: DVec3) -> Option<MoveResult> {
        self.move_frame(mover_type, delta)
    }

    fn refresh_dimensions(&self) {
        self.refresh_dimensions_block_attached_entity();
    }

    fn try_set_position(&self, pos: DVec3) -> Result<(), EntityMoveError> {
        self.try_set_position_block_attached_entity(pos)
    }

    fn interact(
        &self,
        player: &Player,
        hand: InteractionHand,
        _location: DVec3,
    ) -> InteractionResult {
        let held = player.inventory.lock().get_item_in_hand(hand).clone();
        let frame_has_item = !self.item().is_empty();
        let has_held_item = !held.is_empty();
        if self.is_fixed() {
            return InteractionResult::Pass;
        }

        if !frame_has_item {
            if !has_held_item || self.is_removed() {
                return InteractionResult::Pass;
            }
            // Vanilla rejects maps that already track the 256-entity limit.
            // Map saved data is not implemented yet (V05.07), so no map can
            // reach that limit here.
            self.set_item(held);
            self.game_event_with_source_entity(
                &vanilla_game_events::BLOCK_CHANGE,
                Some(player.as_entity_event_source()),
            );
            player.inventory.lock().mutate_item_in_hand(hand, |item| {
                item.consume_one(player.has_infinite_materials());
            });
            return InteractionResult::Success;
        }

        self.play_sound(self.rotate_item_sound(), 1.0, 1.0);
        self.set_rotation(self.rotation() + 1, true);
        self.game_event_with_source_entity(
            &vanilla_game_events::BLOCK_CHANGE,
            Some(player.as_entity_event_source()),
        );
        InteractionResult::Success
    }

    fn save_additional(&self, nbt: &mut NbtCompound) {
        self.save_block_attached_entity(nbt);

        let item = self.item();
        if !item.is_empty() {
            nbt.insert("Item", item.to_nbt_tag_ref());
        }
        let state = *self.state.lock();
        nbt.insert("ItemRotation", self.rotation() as i8);
        nbt.insert("ItemDropChance", state.drop_chance);
        nbt.insert("Facing", self.direction().get_3d_data_value() as i8);
        nbt.insert("Invisible", i8::from(self.is_invisible()));
        nbt.insert("Fixed", i8::from(state.fixed));
    }

    fn load_additional(&self, nbt: BorrowedNbtCompoundView<'_, '_>) {
        self.load_block_attached_entity(nbt);

        let item = nbt
            .compound("Item")
            .and_then(|item_tag| ItemStack::from_borrowed_compound(&item_tag))
            .unwrap_or_else(ItemStack::empty);
        self.set_item_with_update(item, false);

        self.set_rotation(
            i32::from(read_frame_byte(&nbt, "ItemRotation").unwrap_or(0)),
            false,
        );
        self.state.lock().drop_chance = nbt
            .get("ItemDropChance")
            .and_then(|tag| tag.codec_f32())
            .unwrap_or(1.0);

        let facing = nbt
            .get("Facing")
            .and_then(|tag| tag.codec_i32())
            .map_or(Direction::Down, |value| {
                Direction::from_3d_data_value(i32::from(value as i8))
            });
        self.set_direction(facing);

        self.entity_data.set_base_invisible_flag(
            read_frame_byte(&nbt, "Invisible").is_some_and(|value| value != 0),
        );
        self.state.lock().fixed = read_frame_byte(&nbt, "Fixed").is_some_and(|value| value != 0);

        self.recalculate_position_or_warn();
    }
}

impl BlockAttachedEntity for ItemFrameEntity {
    fn block_attached_entity_base(&self) -> &BlockAttachedEntityBase {
        &self.block_attached_entity_base
    }

    fn survives(&self) -> bool {
        ItemFrameEntity::survives(self)
    }

    fn drop_item(&self, caused_by: Option<&dyn Entity>) {
        self.play_sound(self.break_sound(), 1.0, 1.0);
        self.drop_frame_and_contents(caused_by, true);
        self.game_event_with_source_entity(&vanilla_game_events::BLOCK_CHANGE, caused_by);
    }

    fn recalculate_bounding_box(&self) -> Result<(), EntityMoveError> {
        self.try_recalculate_position()
    }
}

fn read_frame_byte(nbt: &BorrowedNbtCompoundView<'_, '_>, key: &str) -> Option<i8> {
    let tag = nbt.get(key)?;
    // TagValueInput byte/boolean access uses NumericTag.byteValue, which floors
    // floating tags; Codec.BYTE (used for Facing) uses Number.byteValue instead.
    if let Some(value) = tag.float() {
        return Some(value.floor() as i32 as i8);
    }
    if let Some(value) = tag.double() {
        return Some(value.floor() as i32 as i8);
    }
    tag.codec_i32().map(|value| value as i8)
}

const fn direction_2d_data_value(direction: Direction) -> u8 {
    match direction {
        Direction::South | Direction::Down | Direction::Up => 0,
        Direction::West => 1,
        Direction::North => 2,
        Direction::East => 3,
    }
}

#[cfg(test)]
mod tests;
