//! Vanilla `DispenseItemBehavior.bootStrap` bucket registrations.
//!
//! Filled buckets are `DispensibleContainerItem`s, so the dispenser empties them
//! into the block in front and keeps the empty bucket. The empty bucket is a
//! `BucketPickup` consumer instead: it fills itself from that block.

use vecta_registry::item_stack::ItemStack;
use vecta_registry::vanilla_game_events;
use vecta_registry::vanilla_items;

use super::dispense::{
    BlockSource, DEFAULT_BEHAVIOR, DefaultDispenseItemBehavior, DispenseItemBehavior,
};
use crate::behavior::ITEM_BEHAVIORS;
use crate::behavior::items::pickup_bucket_content;
use crate::world::game_event::GameEventContext;

/// Vanilla `filledBucketBehavior`: empty the bucket in front of the dispenser and
/// leave an empty bucket behind.
///
/// Registered for every bucket whose item empties a container, which is the
/// water, lava and powder-snow buckets here.
pub struct FilledBucketDispenseBehavior;

impl DispenseItemBehavior for FilledBucketDispenseBehavior {
    fn dispense(&self, source: &BlockSource<'_>, stack: ItemStack) -> ItemStack {
        let result = Self::execute(source, stack);
        DefaultDispenseItemBehavior::play_default_feedback(source);
        result
    }
}

impl FilledBucketDispenseBehavior {
    fn execute(source: &BlockSource<'_>, stack: ItemStack) -> ItemStack {
        let target = source.pos.relative(source.facing());
        let Some(container) = ITEM_BEHAVIORS
            .get_behavior(stack.item())
            .as_dispensible_container()
        else {
            // Vanilla casts the item to `DispensibleContainerItem`; an item
            // without that capability cannot be registered for this behavior.
            return DEFAULT_BEHAVIOR.dispense(source, stack);
        };

        if !container.empty_contents(None, source.world, target, None) {
            return DEFAULT_BEHAVIOR.dispense(source, stack);
        }

        // Vanilla calls `checkExtraContent` here; every bucket item inherits the
        // no-op `DispensibleContainerItem` default.
        DefaultDispenseItemBehavior::consume_with_remainder(
            source,
            stack,
            ItemStack::new(&vanilla_items::BUCKET),
        )
    }
}

/// Vanilla `Items.BUCKET` dispenser behavior: fill the bucket from the block in
/// front of the dispenser.
pub struct EmptyBucketDispenseBehavior;

impl DispenseItemBehavior for EmptyBucketDispenseBehavior {
    fn dispense(&self, source: &BlockSource<'_>, stack: ItemStack) -> ItemStack {
        let result = Self::execute(source, stack);
        DefaultDispenseItemBehavior::play_default_feedback(source);
        result
    }
}

impl EmptyBucketDispenseBehavior {
    fn execute(source: &BlockSource<'_>, stack: ItemStack) -> ItemStack {
        let target = source.pos.relative(source.facing());
        let state = source.world.get_block_state(target);
        let Some(pickup) = pickup_bucket_content(source.world, target, state, None) else {
            return DefaultDispenseItemBehavior::execute(source, stack);
        };

        if pickup.filled_bucket.is_empty() {
            return DefaultDispenseItemBehavior::execute(source, stack);
        }

        // Vanilla fires only `FLUID_PICKUP` here: the fill sound belongs to the
        // player path in `BucketItem.use`.
        source.world.game_event(
            &vanilla_game_events::FLUID_PICKUP,
            target,
            &GameEventContext::new(None, None),
        );
        DefaultDispenseItemBehavior::consume_with_remainder(
            source,
            stack,
            ItemStack::new(pickup.filled_bucket.item()),
        )
    }
}
