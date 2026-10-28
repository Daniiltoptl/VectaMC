//! Thrown experience bottle: impact effects and directed experience orbs.

use std::sync::Arc;
use std::sync::Weak;

use glam::DVec3;
use simdnbt::borrow::NbtCompound as BorrowedNbtCompoundView;
use simdnbt::owned::NbtCompound;
use vecta_macros::entity_behavior;
use vecta_protocol::packets::game::SoundSource;
use vecta_registry::entity_type::EntityTypeRef;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::items::ItemRef;
use vecta_registry::vanilla_entity_data::ExperienceBottleEntityData;
use vecta_registry::{level_events, vanilla_items};
use vecta_utils::locks::SyncMutex;
use vecta_utils::{DowncastType, DowncastTypeKey};

use crate::entity::entities::ExperienceOrbEntity;
use crate::entity::{
    Entity, EntityBase, EntityBaseLoad, EntitySyncedData, Projectile, ProjectileBase,
    ProjectileHit, RemovalReason, SharedEntity, ThrowableItemProjectile, ThrowableProjectile,
};
use crate::world::World;

const SPLASH_COLOR: i32 = -13_083_194;

/// A thrown experience bottle.
#[entity_behavior(class = "ThrownExperienceBottle")]
pub struct ThrownExperienceBottleEntity {
    /// Common entity fields (id, uuid, position, etc.).
    base: EntityBase,
    /// Vanilla entity type registered for this implementation.
    entity_type: EntityTypeRef,
    /// Synced data carrying the rendered item stack.
    entity_data: SyncMutex<ExperienceBottleEntityData>,
    /// Shared `Projectile` state (owner / left-owner / has-been-shot).
    projectile_base: ProjectileBase,
}

// SAFETY: This key is owned by Vecta and uniquely identifies `ThrownExperienceBottleEntity`.
unsafe impl DowncastType for ThrownExperienceBottleEntity {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:entity/thrown_experience_bottle");
}

impl ThrownExperienceBottleEntity {
    /// Creates a new thrown experience bottle with no owner and the default rendered item.
    #[must_use]
    pub fn new(entity_type: EntityTypeRef, id: i32, position: DVec3, world: Weak<World>) -> Self {
        Self {
            base: EntityBase::new(id, position, entity_type.dimensions, world),
            entity_type,
            entity_data: SyncMutex::new(Self::initial_entity_data()),
            projectile_base: ProjectileBase::new(),
        }
    }

    /// Creates a thrown experience bottle from saved base data.
    #[must_use]
    pub fn from_saved(entity_type: EntityTypeRef, load: EntityBaseLoad) -> Self {
        Self {
            base: EntityBase::from_load(load, entity_type.dimensions),
            entity_type,
            entity_data: SyncMutex::new(Self::initial_entity_data()),
            projectile_base: ProjectileBase::new(),
        }
    }

    fn initial_entity_data() -> ExperienceBottleEntityData {
        let mut data = ExperienceBottleEntityData::new();
        data.throwable_item_projectile
            .item_stack
            .set(ItemStack::new(&vanilla_items::EXPERIENCE_BOTTLE));
        data
    }
}

impl Entity for ThrownExperienceBottleEntity {
    fn base(&self) -> &EntityBase {
        &self.base
    }

    fn entity_type(&self) -> EntityTypeRef {
        self.entity_type
    }

    fn tick(self: Arc<Self>) {
        self.throwable_projectile_tick();
    }

    fn get_default_gravity(&self) -> f64 {
        0.07
    }

    fn sound_source(&self) -> SoundSource {
        SoundSource::Neutral
    }

    fn spawn_data(&self) -> i32 {
        self.get_owner().map_or(0, |owner| owner.id())
    }

    fn restore_owner_reference(&self, owner: &SharedEntity) {
        self.cache_owner_entity(owner);
    }

    fn projectile_owner_uuid(&self) -> Option<uuid::Uuid> {
        self.owner_uuid()
    }

    fn projectile_owner(&self) -> Option<SharedEntity> {
        self.get_owner()
    }

    fn synced_data(&self) -> Option<&dyn EntitySyncedData> {
        Some(&self.entity_data)
    }

    fn save_additional(&self, nbt: &mut NbtCompound) {
        self.save_projectile(nbt);
        self.save_throwable_item(nbt);
    }

    fn load_additional(&self, nbt: BorrowedNbtCompoundView<'_, '_>) {
        self.load_projectile(nbt);
        self.load_throwable_item(nbt);
    }
}

impl Projectile for ThrownExperienceBottleEntity {
    fn projectile_base(&self) -> &ProjectileBase {
        &self.projectile_base
    }

    fn on_hit(self: Arc<Self>, hit: &ProjectileHit) {
        Arc::clone(&self).projectile_on_hit(hit);
        let Some(world) = self.level() else {
            return;
        };
        world.level_event(
            level_events::PARTICLES_SPELL_POTION_SPLASH,
            self.block_position(),
            SPLASH_COLOR,
            None,
        );
        if !self.is_silent() {
            world.level_event(
                level_events::SOUND_SPELL_POTION_SPLASH,
                self.block_position(),
                0,
                None,
            );
        }
        let amount = 3 + rand::random_range(0..5) + rand::random_range(0..5);
        let direction = match hit {
            ProjectileHit::Block { hit, .. } => hit.direction.offset_vec().as_dvec3(),
            ProjectileHit::Entity(_) => -self.velocity(),
        };
        ExperienceOrbEntity::award_with_direction(&world, hit.location(), direction, amount);
        self.set_removed(RemovalReason::Discarded);
    }
}

impl ThrowableProjectile for ThrownExperienceBottleEntity {}

impl ThrowableItemProjectile for ThrownExperienceBottleEntity {
    fn get_default_item(&self) -> ItemRef {
        &vanilla_items::EXPERIENCE_BOTTLE
    }

    fn set_item(&self, item: ItemStack) {
        self.entity_data
            .lock()
            .throwable_item_projectile
            .item_stack
            .set(item);
    }

    fn get_item(&self) -> ItemStack {
        self.entity_data
            .lock()
            .throwable_item_projectile
            .item_stack
            .get()
            .clone()
    }
}

#[cfg(test)]
mod tests;
