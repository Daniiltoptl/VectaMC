use vecta_registry::item_stack::ItemStack;
use vecta_registry::level_events;
use vecta_utils::WorldAabb;

use super::dispense::{BlockSource, DEFAULT_BEHAVIOR, DispenseItemBehavior};

pub static EQUIPMENT_BEHAVIOR: EquipmentDispenseItemBehavior = EquipmentDispenseItemBehavior;

pub struct EquipmentDispenseItemBehavior;

impl DispenseItemBehavior for EquipmentDispenseItemBehavior {
    fn dispense(&self, source: &BlockSource<'_>, mut stack: ItemStack) -> ItemStack {
        let pos = source.pos.relative(source.facing());
        let bounds = WorldAabb::new(
            f64::from(pos.x()),
            f64::from(pos.y()),
            f64::from(pos.z()),
            f64::from(pos.x() + 1),
            f64::from(pos.y() + 1),
            f64::from(pos.z() + 1),
        );
        let entities = source
            .world
            .get_entities_in_aabb_matching(&bounds, |entity| {
                entity
                    .as_living_entity()
                    .is_some_and(|living| living.can_equip_with_dispenser(&stack))
            });
        let Some(target) = entities
            .first()
            .and_then(|entity| entity.as_living_entity())
        else {
            return DEFAULT_BEHAVIOR.dispense(source, stack);
        };

        let slot = target.get_equipment_slot_for_item(&stack);
        target.set_item_slot(slot, stack.split(1));
        if let Some(mob) = target.as_mob() {
            mob.set_guaranteed_drop(slot);
            mob.set_persistence_required();
        }

        source
            .world
            .level_event(level_events::SOUND_DISPENSER_DISPENSE, source.pos, 0, None);
        source.world.level_event(
            level_events::PARTICLES_SHOOT_SMOKE,
            source.pos,
            source.facing().get_3d_data_value(),
            None,
        );
        stack
    }
}
