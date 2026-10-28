//! Vanilla `BundleItem`: click overrides, selected item and content drops.

use std::sync::Arc;

use vecta_macros::item_behavior;
use vecta_protocol::packets::game::SoundSource;
use vecta_registry::data_components::components::{BundleContentsMut, ItemUseAnimation};
use vecta_registry::data_components::vanilla_components::BUNDLE_CONTENTS;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::sound_events;
use vecta_registry::stat::vanilla_stat_types;

use crate::behavior::{ITEM_BEHAVIORS, InteractionResult, ItemBehavior, UseItemContext};
use crate::entity::{Entity as _, LivingEntity, LivingEntityRef};
use crate::inventory::click::MouseButton;
use crate::inventory::lock::ContainerLockGuard;
use crate::inventory::slots::Slot;
use crate::player::Player;
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

    /// Vanilla `BundleItem.overrideStackedOnOther`: the bundle is on the cursor.
    ///
    /// Returns whether the click was handled, in which case the menu skips its
    /// default pickup handling.
    pub(crate) fn override_stacked_on_other(
        bundle: &mut ItemStack,
        slot: &dyn Slot,
        guard: &mut ContainerLockGuard,
        player: &Player,
        button: MouseButton,
    ) -> bool {
        let Some(contents) = bundle.get(BUNDLE_CONTENTS).cloned() else {
            return false;
        };
        let mut contents = contents.as_mutable();
        let other = slot.get_item(guard).clone();

        if button == MouseButton::Left && !other.is_empty() {
            let inserted = Self::try_transfer(&mut contents, slot, guard, player, &other);
            if inserted > 0 {
                Self::play_insert_sound(player);
            } else {
                Self::play_insert_fail_sound(player);
            }
            bundle.set(BUNDLE_CONTENTS, contents.to_immutable());
            return true;
        }

        if button == MouseButton::Right && other.is_empty() {
            if let Some(removed) = contents.remove_one() {
                let count = removed.count();
                let mut remainder = slot.safe_insert(guard, removed, count);
                if remainder.is_empty() {
                    Self::play_remove_one_sound(player);
                } else {
                    contents.try_insert(&mut remainder);
                }
            }
            bundle.set(BUNDLE_CONTENTS, contents.to_immutable());
            return true;
        }

        false
    }

    /// Vanilla `BundleItem.overrideOtherStackedOnMe`: the bundle is in the slot.
    pub(crate) fn override_other_stacked_on_me(
        bundle: &mut ItemStack,
        other: &mut ItemStack,
        slot: &dyn Slot,
        guard: &mut ContainerLockGuard,
        player: &Player,
        button: MouseButton,
    ) -> bool {
        if button == MouseButton::Left && other.is_empty() {
            Self::toggle_selected_item(bundle, -1);
            return false;
        }

        let Some(contents) = bundle.get(BUNDLE_CONTENTS).cloned() else {
            return false;
        };
        let mut contents = contents.as_mutable();
        let allow_modification = slot.allow_modification(guard, player);

        if button == MouseButton::Left && !other.is_empty() {
            let inserted = if allow_modification && Self::can_item_be_in_bundle(other) {
                contents.try_insert(other)
            } else {
                0
            };
            if inserted > 0 {
                Self::play_insert_sound(player);
            } else {
                Self::play_insert_fail_sound(player);
            }
            bundle.set(BUNDLE_CONTENTS, contents.to_immutable());
            return true;
        }

        if button == MouseButton::Right && other.is_empty() {
            if allow_modification && let Some(removed) = contents.remove_one() {
                Self::play_remove_one_sound(player);
                *other = removed;
            }
            bundle.set(BUNDLE_CONTENTS, contents.to_immutable());
            return true;
        }

        Self::toggle_selected_item(bundle, -1);
        false
    }

    /// Vanilla `BundleItem.toggleSelectedItem`.
    ///
    /// Returns whether the stack holds bundle contents and was therefore touched.
    pub(crate) fn toggle_selected_item(bundle: &mut ItemStack, selected_item: i32) -> bool {
        let Some(contents) = bundle.get(BUNDLE_CONTENTS).cloned() else {
            return false;
        };
        let mut contents = contents.as_mutable();
        contents.toggle_selected_item(selected_item);
        bundle.set(BUNDLE_CONTENTS, contents.to_immutable());
        true
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
    ) -> i32 {
        if !Self::can_item_be_in_bundle(other) {
            return 0;
        }

        let max_amount = contents.max_amount_for(other);
        let mut taken = slot.safe_take(guard, other.count(), max_amount, player);
        if taken.is_empty() {
            return 0;
        }
        contents.try_insert(&mut taken)
    }

    /// Drops one stored stack, mirroring Vanilla `BundleItem.dropContent`.
    fn drop_content(world: &Arc<World>, player: &Player, bundle: &mut ItemStack) -> bool {
        let Some(contents) = bundle.get(BUNDLE_CONTENTS).cloned() else {
            return false;
        };
        if contents.is_empty() {
            return false;
        }

        let mut contents = contents.as_mutable();
        let Some(removed) = contents.remove_one() else {
            return false;
        };
        Self::play_remove_one_sound(player);
        bundle.set(BUNDLE_CONTENTS, contents.to_immutable());

        let _ = player.spawn_dropped_item(removed, true, false);
        world.play_sound_at(
            &sound_events::ITEM_BUNDLE_DROP_CONTENTS,
            SoundSource::Players,
            player.position(),
            0.8,
            Self::random_pitch(),
            None,
        );
        player.award_stat(&vanilla_stat_types::ITEM_USED, bundle.item());
        true
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

impl ItemBehavior for BundleItem {
    fn as_bundle(&self) -> Option<&BundleItem> {
        Some(self)
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
        if first_tick || repeated_tick {
            Self::drop_content(world, player, stack);
        }
    }
}
