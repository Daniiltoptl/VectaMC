//! Vanilla `Items.BONE_MEAL` dispenser behavior.
//!
//! The dispenser grows the block in front of it through the same
//! `BoneMealItem.growCrop` / `growWaterPlant` contract the hand path uses, then
//! reports 1505 and the dispenser sound pair.

use vecta_registry::item_stack::ItemStack;
use vecta_registry::level_events;

use super::dispense::{BlockSource, DispenseItemBehavior};
use crate::behavior::items::BoneMealItem;

/// Vanilla `OptionalDispenseItemBehavior` for bone meal.
pub struct BoneMealDispenseBehavior;

impl DispenseItemBehavior for BoneMealDispenseBehavior {
    fn dispense(&self, source: &BlockSource<'_>, mut stack: ItemStack) -> ItemStack {
        let target = source.pos.relative(source.facing());
        let mut success = BoneMealItem::grow_crop(source.world, target);
        if success {
            stack.shrink(1);
        } else if BoneMealItem::grow_water_plant(source.world, target, None) {
            // The dispenser has no clicked face, so vanilla passes `null` and
            // the water-plant branch cannot pick a wall coral.
            stack.shrink(1);
            success = true;
        }

        if success {
            source.world.level_event(
                level_events::PARTICLES_AND_SOUND_PLANT_GROWTH,
                target,
                15,
                None,
            );
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
