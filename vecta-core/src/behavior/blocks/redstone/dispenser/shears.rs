//! Vanilla `ShearsDispenseItemBehavior`.
//!
//! The dispenser shears a full beehive or the first shearable entity in front of
//! it. The beehive branch needs a honeycomb loot drop that Vecta does not have
//! yet, so only the entity branch is wired; the hive stays for G114.

use vecta_protocol::packets::game::SoundSource;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::level_events;
use vecta_registry::vanilla_game_events;
use vecta_utils::WorldAabb;

use super::dispense::{BlockSource, DispenseItemBehavior};
use crate::world::game_event::GameEventContext;

/// Vanilla `ShearsDispenseItemBehavior`.
pub struct ShearsDispenseItemBehavior;

impl DispenseItemBehavior for ShearsDispenseItemBehavior {
    fn dispense(&self, source: &BlockSource<'_>, mut stack: ItemStack) -> ItemStack {
        let target = source.pos.relative(source.facing());
        let bounds = WorldAabb::new(
            f64::from(target.x()),
            f64::from(target.y()),
            f64::from(target.z()),
            f64::from(target.x() + 1),
            f64::from(target.y() + 1),
            f64::from(target.z() + 1),
        );
        let entities = source
            .world
            .get_entities_in_aabb_matching(&bounds, |entity| !entity.is_spectator());

        let success = entities.iter().any(|entity| {
            if entity.shear_off_all_leash_connections(None) {
                return true;
            }
            let Some(shearable) = entity.as_shearable() else {
                return false;
            };
            if !entity.is_alive() || !shearable.ready_for_shearing() {
                return false;
            }
            shearable.shear(source.world, SoundSource::Blocks, &stack);
            source.world.game_event(
                &vanilla_game_events::SHEAR,
                target,
                &GameEventContext::new(None, None),
            );
            true
        });

        if success {
            stack.hurt_and_break(1, false);
        }

        source.world.level_event(
            if success {
                level_events::SOUND_DISPENSER_DISPENSE
            } else {
                level_events::SOUND_DISPENSER_FAIL
            },
            source.pos,
            0,
            None,
        );
        source.world.level_event(
            level_events::PARTICLES_SHOOT_SMOKE,
            source.pos,
            source.facing().get_3d_data_value(),
            None,
        );
        stack
    }
}
