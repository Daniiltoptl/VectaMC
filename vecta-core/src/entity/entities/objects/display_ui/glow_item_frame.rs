//! Vanilla `GlowItemFrame`: an `ItemFrame` subtype with glow-specific sounds and item.

use std::sync::{Arc, Weak};

use glam::DVec3;
use simdnbt::borrow::NbtCompound as BorrowedNbtCompoundView;
use simdnbt::owned::NbtCompound;
use vecta_macros::entity_behavior;
use vecta_registry::entity_type::EntityTypeRef;
use vecta_registry::item_stack::ItemStack;
use vecta_utils::types::InteractionHand;
use vecta_utils::{BlockPos, Direction, DowncastType, DowncastTypeKey};

use super::ItemFrameEntity;
use crate::behavior::InteractionResult;
use crate::entity::block_attached_entity::{BlockAttachedEntity, BlockAttachedEntityBase};
use crate::entity::damage::DamageSource;
use crate::entity::{
    Entity, EntityBase, EntityBaseLoad, EntityMoveError, EntitySyncedData, ItemFrame, SharedEntity,
};
use crate::physics::{MoveResult, MoverType};
use crate::player::Player;
use crate::world::World;

/// Vanilla `GlowItemFrame`.
///
/// The subtype only overrides sounds, the dropped frame item and the visual
/// glow; every other behavior comes from the shared [`ItemFrameEntity`].
#[entity_behavior(class = "GlowItemFrame")]
pub struct GlowItemFrameEntity {
    inner: ItemFrameEntity,
}

// SAFETY: This key is owned by Vecta and uniquely identifies `GlowItemFrameEntity`.
unsafe impl DowncastType for GlowItemFrameEntity {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:entity/glow_item_frame");
}

impl GlowItemFrameEntity {
    /// Creates a fresh glow item frame from the generic entity factory path.
    #[must_use]
    pub fn new(entity_type: EntityTypeRef, id: i32, position: DVec3, world: Weak<World>) -> Self {
        Self {
            inner: ItemFrameEntity::new(entity_type, id, position, world),
        }
    }

    /// Creates a fresh glow item frame attached to `block_pos` and facing `direction`.
    #[must_use]
    pub fn new_attached(
        entity_type: EntityTypeRef,
        id: i32,
        block_pos: BlockPos,
        direction: Direction,
        world: Weak<World>,
    ) -> Self {
        Self {
            inner: ItemFrameEntity::new_attached(entity_type, id, block_pos, direction, world),
        }
    }

    /// Creates a glow item frame from persistent entity data.
    #[must_use]
    pub fn from_saved(entity_type: EntityTypeRef, load: EntityBaseLoad) -> Self {
        Self {
            inner: ItemFrameEntity::from_saved(entity_type, load),
        }
    }

    /// Plays the placement sound of this frame variant.
    pub fn play_placement_sound(&self) {
        self.inner.play_placement_sound();
    }

    /// Returns the glow item frame item stack.
    #[must_use]
    pub fn frame_item_stack(&self) -> ItemStack {
        self.inner.frame_item_stack()
    }
}

impl ItemFrame for GlowItemFrameEntity {
    fn direction(&self) -> Direction {
        self.inner.direction()
    }

    fn analog_output(&self) -> i32 {
        self.inner.analog_output()
    }

    fn play_placement_sound(&self) {
        self.inner.play_placement_sound();
    }
}

impl Entity for GlowItemFrameEntity {
    fn base(&self) -> &EntityBase {
        self.inner.base()
    }

    fn entity_type(&self) -> EntityTypeRef {
        self.inner.entity_type()
    }

    fn spawn_data(&self) -> i32 {
        self.inner.spawn_data()
    }

    fn spawn_position(&self) -> DVec3 {
        self.inner.spawn_position()
    }

    fn synced_data(&self) -> Option<&dyn EntitySyncedData> {
        self.inner.synced_data()
    }

    fn tick(self: Arc<Self>) {
        self.inner.tick_block_attached_entity();
    }

    fn is_pickable(&self) -> bool {
        self.inner.is_pickable()
    }

    fn hurt(&self, world: &World, source: &DamageSource, amount: f32) -> bool {
        self.inner.hurt(world, source, amount)
    }

    fn skip_attack_interaction(&self, source: &SharedEntity) -> bool {
        self.inner.skip_attack_interaction(source)
    }

    fn push_impulse(&self, impulse: DVec3) {
        self.inner.push_impulse(impulse);
    }

    fn move_entity(self: Arc<Self>, mover_type: MoverType, delta: DVec3) -> Option<MoveResult> {
        self.inner.move_frame(mover_type, delta)
    }

    fn refresh_dimensions(&self) {
        self.inner.refresh_dimensions();
    }

    fn try_set_position(&self, pos: DVec3) -> Result<(), EntityMoveError> {
        self.inner.try_set_position(pos)
    }

    fn interact(
        &self,
        player: &Player,
        hand: InteractionHand,
        location: DVec3,
    ) -> InteractionResult {
        self.inner.interact(player, hand, location)
    }

    fn save_additional(&self, nbt: &mut NbtCompound) {
        self.inner.save_additional(nbt);
    }

    fn load_additional(&self, nbt: BorrowedNbtCompoundView<'_, '_>) {
        self.inner.load_additional(nbt);
    }
}

impl BlockAttachedEntity for GlowItemFrameEntity {
    fn block_attached_entity_base(&self) -> &BlockAttachedEntityBase {
        self.inner.block_attached_entity_base()
    }

    fn survives(&self) -> bool {
        self.inner.survives()
    }

    fn drop_item(&self, caused_by: Option<&dyn Entity>) {
        self.inner.drop_item(caused_by);
    }

    fn recalculate_bounding_box(&self) -> Result<(), EntityMoveError> {
        self.inner.recalculate_bounding_box()
    }
}
