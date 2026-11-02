//! Vanilla `FlintAndSteelDispenseItemBehavior`.
//!
//! The dispenser path differs from `FlintAndSteelItem.useOn`: it acts on the
//! block in front of the dispenser instead of the clicked face, primes TNT
//! directly, and plays no flint sound of its own. `Items.FIRE_CHARGE` is a
//! projectile registration instead and stays unregistered until a
//! `SmallFireball` entity exists.

use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::blocks::properties::BlockStateProperties;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::level_events;
use vecta_registry::{vanilla_blocks, vanilla_game_events};
use vecta_utils::types::UpdateFlags;

use super::dispense::{BlockSource, DispenseItemBehavior};
use crate::behavior::blocks::{FireBlock, TntBlock};
use crate::behavior::items::can_light;
use crate::world::game_event::GameEventContext;

/// Vanilla `FlintAndSteelDispenseItemBehavior`.
pub struct FlintAndSteelDispenseBehavior;

impl DispenseItemBehavior for FlintAndSteelDispenseBehavior {
    fn dispense(&self, source: &BlockSource<'_>, mut stack: ItemStack) -> ItemStack {
        let facing = source.facing();
        let target = source.pos.relative(facing);
        let state = source.world.get_block_state(target);
        // Vanilla starts successful and only clears the flag when TNT refuses
        // to prime, so a flint that lights nothing still wears out.
        let mut success = true;

        if FireBlock::can_be_placed_at(source.world, target, facing) {
            source.world.set_block(
                target,
                FireBlock::get_state(source.world.as_ref(), target),
                UpdateFlags::UPDATE_ALL,
            );
            source.world.game_event(
                &vanilla_game_events::BLOCK_PLACE,
                target,
                &GameEventContext::new(None, None),
            );
        } else if can_light(state) {
            source.world.set_block(
                target,
                state.set_value(&BlockStateProperties::LIT, true),
                UpdateFlags::UPDATE_ALL,
            );
            source.world.game_event(
                &vanilla_game_events::BLOCK_CHANGE,
                target,
                &GameEventContext::new(None, None),
            );
        } else if state.get_block() == &vanilla_blocks::TNT {
            if TntBlock::prime(source.world, target) {
                source.world.set_block(
                    target,
                    vanilla_blocks::AIR.default_state(),
                    UpdateFlags::UPDATE_ALL,
                );
            } else {
                success = false;
            }
        }

        if success {
            // Vanilla `hurtAndBreak(1, level, null, ...)`: a dispenser has no
            // player, so nothing is exempt as in creative.
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
            facing.get_3d_data_value(),
            None,
        );
        stack
    }
}
