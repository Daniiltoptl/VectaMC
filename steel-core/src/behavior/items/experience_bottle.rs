//! Experience bottle throwing from either hand.

use std::sync::Arc;

use steel_macros::item_behavior;
use steel_protocol::packets::game::SoundSource;
use steel_registry::{sound_events, vanilla_entities};

use super::throw_projectile::{ThrowParams, throw_item_projectile};
use crate::behavior::{InteractionResult, ItemBehavior, UseItemContext};
use crate::entity::entities::ThrownExperienceBottleEntity;
use crate::entity::next_entity_id;

const THROW: ThrowParams = ThrowParams {
    sound: &sound_events::ENTITY_EXPERIENCE_BOTTLE_THROW,
    sound_source: SoundSource::Neutral,
    sound_volume: 0.5,
    y_offset: -20.0,
    power: 0.7,
    uncertainty: 1.0,
};

/// Behavior for throwing an experience bottle.
#[item_behavior]
pub struct ExperienceBottleItem;

impl ItemBehavior for ExperienceBottleItem {
    fn use_item(&self, context: &mut UseItemContext) -> InteractionResult {
        let world = context.world;
        throw_item_projectile(context, &THROW, |position| {
            ThrownExperienceBottleEntity::new(
                &vanilla_entities::EXPERIENCE_BOTTLE,
                next_entity_id(),
                position,
                Arc::downgrade(world),
            )
        });
        InteractionResult::Success
    }
}

#[cfg(test)]
mod tests;
