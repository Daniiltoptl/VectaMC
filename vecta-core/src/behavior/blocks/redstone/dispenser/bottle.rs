//! Vanilla `Items.GLASS_BOTTLE` dispenser behavior.
//!
//! Vanilla checks a full beehive first, then any water fluid, and ejects the
//! bottle otherwise. Unlike the hand path, which ray-traces with
//! `ClipContext.Fluid.SOURCE_ONLY`, the dispenser reads the fluid state of the
//! block in front of it, so flowing water fills a bottle too.

use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::blocks::properties::BlockStateProperties;
use vecta_registry::data_components::PotionContents;
use vecta_registry::fluid::FluidStateExt as _;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::level_events;
use vecta_registry::vanilla_block_tags::BlockTag;
use vecta_registry::{vanilla_game_events, vanilla_items, vanilla_potions};

use super::dispense::{BlockSource, DefaultDispenseItemBehavior, DispenseItemBehavior};
use crate::behavior::blocks::container::BeehiveBlock;
use crate::world::game_event::GameEventContext;

/// Vanilla honey level at which a bottle may be filled.
const FULL_HONEY_LEVEL: u8 = 5;

/// Vanilla `OptionalDispenseItemBehavior` for glass bottles.
pub struct GlassBottleDispenseBehavior;

impl DispenseItemBehavior for GlassBottleDispenseBehavior {
    fn dispense(&self, source: &BlockSource<'_>, stack: ItemStack) -> ItemStack {
        let target = source.pos.relative(source.facing());
        let state = source.world.get_block_state(target);

        let honey_level = state.try_get_value(&BlockStateProperties::LEVEL_HONEY);
        let filled = if state.get_block().has_tag(&BlockTag::BEEHIVES)
            && honey_level.is_some_and(|level| level >= FULL_HONEY_LEVEL)
        {
            BeehiveBlock::reset_honey_level(source.world, state, target);
            Some(ItemStack::new(&vanilla_items::HONEY_BOTTLE))
        } else if state.get_fluid_state().is_water() {
            Some(PotionContents::create_item_stack(
                &vanilla_items::POTION,
                &vanilla_potions::WATER,
            ))
        } else {
            None
        };

        let (result, success) = if let Some(filled) = filled {
            // Vanilla `takeLiquid` fires `FLUID_PICKUP` at the dispenser, not
            // at the block that gave up the liquid.
            source.world.game_event(
                &vanilla_game_events::FLUID_PICKUP,
                source.pos,
                &GameEventContext::new(None, None),
            );
            (
                DefaultDispenseItemBehavior::consume_with_remainder(source, stack, filled),
                true,
            )
        } else {
            // Vanilla `super.execute` throws the bottle out; the sound below
            // replaces the default one because the attempt failed.
            let mut remaining = stack;
            let mut one = remaining.clone();
            one.set_count(1);
            remaining.shrink(1);
            DefaultDispenseItemBehavior::spawn_item(source, one);
            (remaining, false)
        };

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
        result
    }
}
