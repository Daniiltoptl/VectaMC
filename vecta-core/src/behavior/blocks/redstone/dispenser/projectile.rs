//! `ProjectileDispenseBehavior`: dispense a stack by launching its projectile.
//!
//! Vanilla's `ProjectileItem` contract has two halves — an item turns a stack
//! into a projectile entity (`asProjectile`) and describes how the dispenser
//! should throw it (`createDispenseConfig`). Vecta keeps the same split: the
//! factory knows how to build and launch the entity, and [`DispenseConfig`]
//! carries the position offset, power, uncertainty and the optional level-event
//! override.

use std::marker::PhantomData;
use std::sync::Arc;

use glam::DVec3;
use vecta_registry::blocks::properties::Direction;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::level_events;

use super::dispense::{BlockSource, DispenseItemBehavior, SPAWN_OFFSET};
use crate::enchantment_helper;
use crate::entity::{Entity, Projectile, SharedEntity, ThrowableItemProjectile};
use crate::world::World;

/// Vanilla `ProjectileItem.DispenseConfig` default power.
pub const DEFAULT_POWER: f32 = 1.1;

/// Vanilla `ProjectileItem.DispenseConfig` default uncertainty.
pub const DEFAULT_UNCERTAINTY: f32 = 6.0;

/// Vanilla `ProjectileItem.DispenseConfig` default position offset, applied on
/// top of `DispenserBlock.getDispensePosition(source, 0.7, …)`.
pub const DEFAULT_POSITION_OFFSET: DVec3 = DVec3::new(0.0, 0.1, 0.0);

/// Vanilla `ProjectileItem.DispenseConfig`.
#[derive(Clone, Copy, Debug)]
pub struct DispenseConfig {
    /// Added to the vanilla dispense position.
    pub position_offset: DVec3,
    /// Passed to `Projectile.shoot` as `uncertainty`.
    pub uncertainty: f32,
    /// Passed to `Projectile.shoot` as `power`.
    pub power: f32,
    /// Replaces the default `1002` launch event when present.
    pub override_dispense_event: Option<i32>,
}

impl DispenseConfig {
    /// Vanilla `ProjectileItem.DispenseConfig.DEFAULT`.
    pub const DEFAULT: Self = Self {
        position_offset: DEFAULT_POSITION_OFFSET,
        uncertainty: DEFAULT_UNCERTAINTY,
        power: DEFAULT_POWER,
        override_dispense_event: None,
    };

    /// Vanilla `DispenserBlock.getDispensePosition(source, 0.7, offset)`.
    #[must_use]
    pub fn position(&self, source: &BlockSource<'_>) -> DVec3 {
        source.center()
            + DVec3::from(source.facing().offset_vec()) * SPAWN_OFFSET
            + self.position_offset
    }
}

/// Builds and launches one dispensed projectile.
///
/// Vanilla reaches the same thing through `ProjectileItem.asProjectile` plus
/// `Projectile.spawnProjectileUsingShoot`; this trait is the seam so a factory
/// can name a concrete entity type while the behavior stays type-erased.
pub trait DispenseProjectileFactory: Send + Sync {
    /// Launches the projectile and returns it even if world insertion is rejected.
    fn spawn(
        &self,
        world: &Arc<World>,
        position: DVec3,
        direction: DVec3,
        stack: &ItemStack,
        power: f32,
        uncertainty: f32,
    ) -> SharedEntity;
}

/// A factory for vanilla throwable item projectiles, which all store the item
/// stack they were thrown from and share the `Projectile.shoot` path.
pub struct ThrowableItemProjectileFactory<E> {
    create: fn(&Arc<World>, DVec3) -> E,
    _entity: PhantomData<fn() -> E>,
}

impl<E> ThrowableItemProjectileFactory<E> {
    /// Creates a factory from the entity constructor.
    #[must_use]
    pub const fn new(create: fn(&Arc<World>, DVec3) -> E) -> Self {
        Self {
            create,
            _entity: PhantomData,
        }
    }
}

impl<E> DispenseProjectileFactory for ThrowableItemProjectileFactory<E>
where
    E: Projectile + ThrowableItemProjectile + Entity + 'static,
{
    fn spawn(
        &self,
        world: &Arc<World>,
        position: DVec3,
        direction: DVec3,
        stack: &ItemStack,
        power: f32,
        uncertainty: f32,
    ) -> SharedEntity {
        let entity = (self.create)(world, position);
        // Vanilla passes the stack to the entity constructor, which keeps a
        // single-item copy of it.
        entity.set_item_clamped(stack.clone());
        entity.shoot(direction, power, uncertainty);

        let entity: SharedEntity = Arc::new(entity);
        if let Err(error) = world.try_add_entity(Arc::clone(&entity)) {
            log::debug!("failed to spawn a dispensed projectile: {error}");
        }
        entity
    }
}

/// Vanilla `ProjectileDispenseBehavior`: `execute` launches the projectile,
/// `playSound` emits the launch event, and `playAnimation` still runs the
/// inherited `2000` smoke animation.
pub struct ProjectileDispenseBehavior {
    config: DispenseConfig,
    factory: Box<dyn DispenseProjectileFactory>,
}

impl ProjectileDispenseBehavior {
    /// Creates the behavior for one projectile item.
    #[must_use]
    pub fn new(config: DispenseConfig, factory: impl DispenseProjectileFactory + 'static) -> Self {
        Self {
            config,
            factory: Box::new(factory),
        }
    }
}

impl DispenseItemBehavior for ProjectileDispenseBehavior {
    fn dispense(&self, source: &BlockSource<'_>, stack: ItemStack) -> ItemStack {
        let facing: Direction = source.facing();
        let position = self.config.position(source);
        let direction = DVec3::from(facing.offset_vec());

        let projectile = self.factory.spawn(
            source.world,
            position,
            direction,
            &stack,
            self.config.power,
            self.config.uncertainty,
        );

        let mut remaining = stack;
        enchantment_helper::on_projectile_spawned(
            source.world,
            &mut remaining,
            projectile.as_ref(),
            None,
        );
        remaining.shrink(1);

        source.world.level_event(
            self.config
                .override_dispense_event
                .unwrap_or(level_events::SOUND_DISPENSER_PROJECTILE_LAUNCH),
            source.pos,
            0,
            None,
        );
        source.world.level_event(
            level_events::PARTICLES_SHOOT_SMOKE,
            source.pos,
            facing.get_3d_data_value(),
            None,
        );

        remaining
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_config_matches_vanilla() {
        let config = DispenseConfig::DEFAULT;

        assert!(
            (config.power - 1.1).abs() < f32::EPSILON,
            "vanilla `DispenseConfig.Builder.power` defaults to 1.1"
        );
        assert!(
            (config.uncertainty - 6.0).abs() < f32::EPSILON,
            "vanilla `DispenseConfig.Builder.uncertainty` defaults to 6.0"
        );
        assert_eq!(
            config.override_dispense_event, None,
            "vanilla only overrides the launch event when an item asks for it"
        );
        assert!(
            (config.position_offset - DVec3::new(0.0, 0.1, 0.0)).length() < f64::EPSILON,
            "vanilla's default position function adds `Vec3(0, 0.1, 0)`"
        );
    }
}
