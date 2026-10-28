//! Vanilla `BundleItem`: click overrides, selected item and content drops.

#[cfg(test)]
#[path = "bundle/tests.rs"]
mod tests;

use std::io::Result;
use std::sync::Arc;

use vecta_macros::item_behavior;
use vecta_protocol::packets::game::SoundSource;
use vecta_registry::data_components::components::{
    BundleContents, BundleContentsMut, ItemUseAnimation,
};
use vecta_registry::data_components::vanilla_components::BUNDLE_CONTENTS;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::sound_events;
use vecta_registry::stat::vanilla_stat_types;

use crate::behavior::{
    ITEM_BEHAVIORS, InteractionResult, ItemBehavior, ItemClickBehavior, UseItemContext,
};
use crate::entity::{Entity as _, LivingEntity, LivingEntityRef, entities::ItemEntity};
use crate::inventory::click::MouseButton;
use crate::inventory::lock::ContainerLockGuard;
use crate::inventory::slots::Slot;
use crate::player::{Player, player_inventory::Prediction};
use crate::world::World;

/// Vanilla `BundleItem.getUseDuration`.
const USE_DURATION: i32 = 200;
/// Vanilla `TICKS_AFTER_FIRST_THROW`.
const TICKS_AFTER_FIRST_THROW: i32 = 10;
/// Vanilla `TICKS_BETWEEN_THROWS`.
const TICKS_BETWEEN_THROWS: i32 = 2;

/// Behavior for every data-driven `BundleItem` registry entry.
#[item_behavior(class = "BundleItem")]
pub struct BundleItem;

impl BundleItem {
    /// Returns whether `stack` may be stored in a bundle.
    ///
    /// Vanilla `BundleContents.canItemBeInBundle`.
    pub(crate) fn can_item_be_in_bundle(stack: &ItemStack) -> bool {
        !stack.is_empty()
            && ITEM_BEHAVIORS
                .get_behavior(stack.item())
                .can_fit_inside_container_items()
    }

    /// Vanilla `BundleItem.toggleSelectedItem`.
    ///
    /// Returns whether the stack holds bundle contents and was therefore touched.
    pub(crate) fn toggle_selected_item(bundle: &mut ItemStack, selected_item: i32) -> Result<bool> {
        let Some(contents) = bundle.get(BUNDLE_CONTENTS).cloned() else {
            return Ok(false);
        };
        let mut contents = contents.as_mutable();
        contents.toggle_selected_item(selected_item);
        bundle.set(BUNDLE_CONTENTS, contents.to_immutable()?);
        Ok(true)
    }

    /// Moves as much of `other` into the bundle as fits, returning the amount moved.
    ///
    /// Vanilla `BundleContents.Mutable.tryTransfer`.
    fn try_transfer(
        contents: &mut BundleContentsMut,
        slot: &dyn Slot,
        guard: &mut ContainerLockGuard,
        player: &Player,
        other: &ItemStack,
    ) -> Result<i32> {
        let max_amount = contents.max_amount_for(other)?;
        if !Self::can_item_be_in_bundle(other) {
            return Ok(0);
        }

        let mut taken = slot.safe_take(guard, other.count(), max_amount, player);
        if taken.is_empty() {
            return Ok(0);
        }
        contents.try_insert(&mut taken)
    }

    /// Drops one stored stack, mirroring Vanilla `BundleItem.dropContent`.
    fn drop_content(world: &Arc<World>, player: &Player, bundle: &mut ItemStack) -> Result<bool> {
        let Some(contents) = bundle.get(BUNDLE_CONTENTS).cloned() else {
            return Ok(false);
        };
        if contents.is_empty() {
            return Ok(false);
        }

        let mut contents = contents.as_mutable();
        let Some(removed) = contents.remove_one()? else {
            return Ok(false);
        };
        Self::play_remove_one_sound(player);
        bundle.set(BUNDLE_CONTENTS, contents.to_immutable()?);

        let _ = player.drop_item(removed, true, Prediction::Predicted);
        world.play_sound(
            &sound_events::ITEM_BUNDLE_DROP_CONTENTS,
            SoundSource::Players,
            player.block_position(),
            0.8,
            Self::random_pitch(),
            None,
        );
        player.award_stat(&vanilla_stat_types::ITEM_USED, bundle.item());
        Ok(true)
    }

    /// Vanilla picks `0.8 + random * 0.4` for bundle sounds.
    fn random_pitch() -> f32 {
        0.8 + rand::random_range(0.0_f32..1.0) * 0.4
    }

    fn play_insert_sound(player: &Player) {
        player.play_sound(&sound_events::ITEM_BUNDLE_INSERT, 0.8, Self::random_pitch());
    }

    fn play_insert_fail_sound(player: &Player) {
        player.play_sound(&sound_events::ITEM_BUNDLE_INSERT_FAIL, 1.0, 1.0);
    }

    fn play_remove_one_sound(player: &Player) {
        player.play_sound(
            &sound_events::ITEM_BUNDLE_REMOVE_ONE,
            0.8,
            Self::random_pitch(),
        );
    }
}

impl ItemClickBehavior for BundleItem {
    /// Vanilla `BundleItem.overrideStackedOnOther`: the bundle is on the cursor.
    ///
    /// Returns whether the click was handled, in which case the menu skips its
    /// default pickup handling.
    fn override_stacked_on_other(
        &self,
        bundle: &mut ItemStack,
        slot: &dyn Slot,
        guard: &mut ContainerLockGuard,
        player: &Player,
        button: MouseButton,
    ) -> Result<bool> {
        let Some(contents) = bundle.get(BUNDLE_CONTENTS).cloned() else {
            return Ok(false);
        };
        let mut contents = contents.as_mutable();
        let other = slot.get_item(guard).clone();

        if button == MouseButton::Left && !other.is_empty() {
            let inserted = Self::try_transfer(&mut contents, slot, guard, player, &other)?;
            if inserted > 0 {
                Self::play_insert_sound(player);
            } else {
                Self::play_insert_fail_sound(player);
            }
            bundle.set(BUNDLE_CONTENTS, contents.to_immutable()?);
            return Ok(true);
        }

        if button == MouseButton::Right && other.is_empty() {
            if let Some(removed) = contents.remove_one()? {
                let count = removed.count();
                let mut remainder = slot.safe_insert(guard, removed, count);
                if remainder.is_empty() {
                    Self::play_remove_one_sound(player);
                } else if Self::can_item_be_in_bundle(&remainder) {
                    contents.try_insert(&mut remainder)?;
                }
            }
            bundle.set(BUNDLE_CONTENTS, contents.to_immutable()?);
            return Ok(true);
        }

        Ok(false)
    }

    /// Vanilla `BundleItem.overrideOtherStackedOnMe`: the bundle is in the slot.
    fn override_other_stacked_on_me(
        &self,
        bundle: &mut ItemStack,
        other: &mut ItemStack,
        slot: &dyn Slot,
        guard: &mut ContainerLockGuard,
        player: &Player,
        button: MouseButton,
    ) -> Result<bool> {
        if button == MouseButton::Left && other.is_empty() {
            Self::toggle_selected_item(bundle, -1)?;
            return Ok(false);
        }

        let Some(contents) = bundle.get(BUNDLE_CONTENTS).cloned() else {
            return Ok(false);
        };
        let mut contents = contents.as_mutable();
        let allow_modification = slot.allow_modification(guard, player);

        if button == MouseButton::Left && !other.is_empty() {
            let inserted = if allow_modification && Self::can_item_be_in_bundle(other) {
                contents.try_insert(other)?
            } else {
                0
            };
            if inserted > 0 {
                Self::play_insert_sound(player);
            } else {
                Self::play_insert_fail_sound(player);
            }
            bundle.set(BUNDLE_CONTENTS, contents.to_immutable()?);
            return Ok(true);
        }

        if button == MouseButton::Right && other.is_empty() {
            if allow_modification && let Some(removed) = contents.remove_one()? {
                Self::play_remove_one_sound(player);
                *other = removed;
            }
            bundle.set(BUNDLE_CONTENTS, contents.to_immutable()?);
            return Ok(true);
        }

        Self::toggle_selected_item(bundle, -1)?;
        Ok(false)
    }
}

impl ItemBehavior for BundleItem {
    fn as_item_click_behavior(&self) -> Option<&dyn ItemClickBehavior> {
        Some(self)
    }

    fn on_destroyed(&self, entity: &ItemEntity) {
        let mut bundle = entity.get_item();
        let Some(contents) = bundle.get(BUNDLE_CONTENTS).cloned() else {
            return;
        };
        bundle.set(BUNDLE_CONTENTS, BundleContents::empty());
        entity.set_item(bundle);
        let Some(world) = entity.level() else {
            return;
        };
        for item in contents.items() {
            let _ = world.spawn_item(entity.position(), item.create());
        }
    }

    fn use_item(&self, context: &mut UseItemContext) -> InteractionResult {
        context.player.start_using_item(context.hand);
        InteractionResult::Success
    }

    fn get_use_animation(&self, _stack: &ItemStack) -> ItemUseAnimation {
        ItemUseAnimation::Bundle
    }

    fn get_use_duration(&self, _stack: &ItemStack, _user: &dyn LivingEntity) -> i32 {
        USE_DURATION
    }

    fn on_use_tick(
        &self,
        world: &Arc<World>,
        user: LivingEntityRef<'_>,
        stack: &mut ItemStack,
        ticks_remaining: i32,
    ) {
        let Some(player) = user.living().as_player() else {
            return;
        };
        let first_tick = ticks_remaining == USE_DURATION;
        let repeated_tick = ticks_remaining < USE_DURATION - TICKS_AFTER_FIRST_THROW
            && ticks_remaining % TICKS_BETWEEN_THROWS == 0;
        if (first_tick || repeated_tick)
            && let Err(error) = Self::drop_content(world, player, stack)
        {
            // Unlike packet clicks, failed use ticks reach Level.guardEntityTick
            // and become fatal entity-tick errors in vanilla.
            panic!("Bundle item use failed: {error}");
        }
    }
}
