//! Component-driven instrument use, including goat horns.

use vecta_macros::item_behavior;
use vecta_protocol::packets::game::SoundSource;
use vecta_registry::data_components::vanilla_components::INSTRUMENT;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::stat::vanilla_stat_types;
use vecta_registry::vanilla_game_events;
use vecta_utils::types::InteractionHand;

use crate::behavior::{InteractionResult, ItemBehavior, ItemUseAnimation, UseItemContext};
use crate::entity::{Entity, LivingEntity};
use crate::inventory::equipment::EquipmentSlot;
use crate::world::game_event::GameEventContext;

/// Plays the instrument carried by the held stack.
#[item_behavior]
pub struct InstrumentItem;

impl ItemBehavior for InstrumentItem {
    fn use_item(&self, context: &mut UseItemContext) -> InteractionResult {
        // Snapshot the component before starting use, which also reads the inventory.
        let (instrument, item_ref) = context
            .inv
            .with_item(|item| (item.get(INSTRUMENT).cloned(), item.item()));
        let Some(instrument) = instrument else {
            return InteractionResult::Fail;
        };
        let value = instrument.value();
        context.player.start_using_item(context.hand);
        context.world.play_sound_holder_at(
            value.sound_event(),
            SoundSource::Records,
            context.player.position(),
            value.range() / 16.0,
            1.0,
            Some(context.player.id()),
        );
        context.world.game_event_at(
            &vanilla_game_events::INSTRUMENT_PLAY,
            context.player.position(),
            &GameEventContext::new(Some(context.player.as_ref()), None),
        );

        let broken = context.inv.with_item(|item| {
            value.durability_damage() > 0
                && item.hurt_and_break(
                    value.durability_damage(),
                    context.player.has_infinite_materials(),
                )
        });
        if broken {
            let slot = match context.hand {
                InteractionHand::MainHand => EquipmentSlot::MainHand,
                InteractionHand::OffHand => EquipmentSlot::OffHand,
            };
            context.player.on_equipped_item_broken(item_ref, slot);
        }
        if value.use_duration() > 0.0 {
            context.inv.with_item(|item| {
                context
                    .player
                    .add_item_cooldown(item, (value.use_duration() * 20.0).floor() as i32);
            });
        }
        context
            .player
            .award_stat(&vanilla_stat_types::ITEM_USED, item_ref);
        InteractionResult::Consume
    }

    fn get_use_duration(&self, stack: &ItemStack, _user: &dyn LivingEntity) -> i32 {
        stack.get(INSTRUMENT).map_or(0, |instrument| {
            (instrument.value().use_duration() * 20.0).floor() as i32
        })
    }

    fn get_use_animation(&self, _stack: &ItemStack) -> ItemUseAnimation {
        ItemUseAnimation::TootHorn
    }
}

#[cfg(test)]
mod tests;
