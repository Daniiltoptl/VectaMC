//! Smithing table menu.
//!
//! Vanilla's `SmithingMenu` is an `ItemCombinerMenu` with three inputs
//! (template, base, addition), a virtual result slot recomputed from the first
//! matching `minecraft:smithing` recipe, and a `hasRecipeError` data slot that
//! lights up `container.upgrade.error_tooltip` when every input is filled but
//! nothing matches.

use std::sync::Arc;

use vecta_registry::blocks::block_state_ext::BlockStateExt;
use vecta_registry::recipe::SmithingRecipeInput;
use vecta_registry::vanilla_blocks;
use vecta_registry::vanilla_menu_types;
use vecta_utils::locks::{IntoShared as _, Shared};
use vecta_utils::{BlockPos, DowncastType, DowncastTypeKey};

use crate::{
    inventory::{
        container::{ResultContainer, SimpleContainer},
        prelude::*,
        recipe_manager::{SmithingItemTests, assemble_smithing, find_smithing_recipe},
        slots::SmithingResultHandler,
    },
    player::player_inventory::{PlayerInventory, Prediction},
    world::World,
};

#[cfg(test)]
mod tests;

/// Builds the smithing table menu.
#[must_use]
pub fn smithing(
    inventory: Shared<PlayerInventory>,
    container_id: u8,
    pos: BlockPos,
    world: &Arc<World>,
) -> Menu {
    let item_tests = Arc::new(SmithingItemTests::collect());
    let input_container = SimpleContainer::new(3).into_shared();
    let result_container = ResultContainer::new().into_shared();

    let mut builder = MenuBuilder::new(&vanilla_menu_types::SMITHING, container_id);

    let placement_tests = Arc::clone(&item_tests);
    let inputs = builder.section_all_with(
        &input_container,
        SectionKind::restricted(move |index, stack| match index {
            0 => placement_tests.accepts_template(stack),
            1 => placement_tests.accepts_base(stack),
            2 => placement_tests.accepts_addition(stack),
            _ => false,
        }),
    );
    let result = builder.result_slot_with(
        SmithingResultHandler::new(
            Arc::clone(&input_container),
            Arc::clone(&result_container),
            pos,
            Arc::clone(world),
        ),
        CraftedStat::TakenStack,
    );

    let player = builder.player_inventory(&inventory);

    let has_recipe_error = builder.data_slot(0);

    builder.route_with_remainder_policy(
        result,
        player.all(),
        FillDirection::Backward,
        FakeResultRemainderPolicy::Discard,
    );
    builder.route(inputs, player.all(), FillDirection::Forward);
    builder.route(player.hotbar(), inputs, FillDirection::Forward);
    builder.route(player.main(), inputs, FillDirection::Forward);
    builder.drain(inputs);

    builder.build(SmithingKind {
        input_container,
        result_container,
        item_tests,
        block_pos: pos,
        world: Arc::clone(world),
        inputs,
        result,
        player,
        has_recipe_error,
    })
}

/// Per-menu smithing state: the three inputs, the derived result, and the
/// recipe-error flag.
pub struct SmithingKind {
    input_container: Shared<SimpleContainer>,
    result_container: Shared<ResultContainer>,
    item_tests: Arc<SmithingItemTests>,
    block_pos: BlockPos,
    world: Arc<World>,
    inputs: Section,
    result: Section,
    player: PlayerInventorySections,
    has_recipe_error: DataSlot,
}

// SAFETY: This Vecta-owned key uniquely identifies the concrete menu kind
// within the process.
unsafe impl DowncastType for SmithingKind {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:menu/smithing");
}

impl SmithingKind {
    /// Builds the result from the current three inputs, matching
    /// `ItemCombinerMenu.createResult`.
    fn create_result(&self, guard: &mut ContainerLockGuard) {
        let Some(inputs) = guard.get(ContainerId::from_arc(&self.input_container)) else {
            log::warn!("input container not locked while updating the smithing result");
            return;
        };
        let input = SmithingRecipeInput::new(
            inputs.get_item(0).clone(),
            inputs.get_item(1).clone(),
            inputs.get_item(2).clone(),
        );
        let result = find_smithing_recipe(&input)
            .map_or_else(ItemStack::empty, |recipe| assemble_smithing(recipe, &input));

        let Some(result_container) = guard.get_mut(ContainerId::from_arc(&self.result_container))
        else {
            log::warn!("result container not locked while updating the smithing result");
            return;
        };
        result_container.set_item(0, result);
    }

    /// Mirrors `SmithingMenu.slotsChanged`'s error flag: every input filled and
    /// no result.
    fn update_recipe_error(&self, behavior: &mut MenuBehavior, guard: &ContainerLockGuard) {
        let all_inputs_filled = guard
            .get(ContainerId::from_arc(&self.input_container))
            .is_some_and(|inputs| {
                (0..self.inputs.len()).all(|slot| !inputs.get_item(slot).is_empty())
            });
        let result_empty = guard
            .get(ContainerId::from_arc(&self.result_container))
            .is_none_or(|result| result.get_item(0).is_empty());
        self.has_recipe_error
            .set(behavior, i16::from(all_inputs_filled && result_empty));
    }

    /// Mirrors `SmithingMenu.canMoveIntoInputSlots`.
    fn can_move_into_input_slots(&self, guard: &ContainerLockGuard, stack: &ItemStack) -> bool {
        let Some(inputs) = guard.get(ContainerId::from_arc(&self.input_container)) else {
            return false;
        };
        (self.item_tests.accepts_template(stack) && inputs.get_item(0).is_empty())
            || (self.item_tests.accepts_base(stack) && inputs.get_item(1).is_empty())
            || (self.item_tests.accepts_addition(stack) && inputs.get_item(2).is_empty())
    }
}

impl MenuKind for SmithingKind {
    /// Returns true while the original smithing table remains in range.
    fn still_valid(&self, _behavior: &MenuBehavior, player: &Player) -> bool {
        let state = self.world.get_block_state(self.block_pos);
        state.get_block() == &vanilla_blocks::SMITHING_TABLE
            && player.is_within_block_interaction_range_with_buffer(self.block_pos, 4.0)
    }

    fn slots_changed(
        &mut self,
        behavior: &mut MenuBehavior,
        guard: &mut ContainerLockGuard,
        _player: &Player,
    ) {
        // `ItemCombinerMenu.slotsChanged` recomputes the result first; the error
        // flag is then derived from the fresh result.
        self.create_result(guard);
        self.update_recipe_error(behavior, guard);
    }

    /// Clears the virtual result on close. The inputs are drained by [`Menu::removed`].
    fn removed(&mut self, _behavior: &mut MenuBehavior, _player: &Player) {
        self.result_container.lock().set_item(0, ItemStack::empty());
    }

    fn can_take_item_for_pick_all(&self, _carried: &ItemStack, slot_index: usize) -> bool {
        !self.result.contains(slot_index)
    }

    /// Mirrors `ItemCombinerMenu.quickMoveStack`.
    fn quick_move(
        &mut self,
        behavior: &mut MenuBehavior,
        guard: &mut ContainerLockGuard,
        slot_index: usize,
        player: &Player,
    ) -> Option<ItemStack> {
        if slot_index >= behavior.slots().len() {
            return Some(ItemStack::empty());
        }
        let clicked = behavior.slots()[slot_index].get_item(guard).clone();
        if clicked.is_empty() {
            return Some(ItemStack::empty());
        }
        let mut remaining = clicked.clone();

        let moved = if self.result.contains(slot_index) {
            behavior.move_item_stack_to(
                guard,
                slot_index,
                &mut remaining,
                self.player.all().start(),
                self.player.all().end(),
                FillDirection::Backward,
            )
        } else if self.inputs.contains(slot_index) {
            behavior.move_item_stack_to(
                guard,
                slot_index,
                &mut remaining,
                self.player.all().start(),
                self.player.all().end(),
                FillDirection::Forward,
            )
        } else if self.can_move_into_input_slots(guard, &clicked)
            && self.player.all().contains(slot_index)
        {
            behavior.move_item_stack_to(
                guard,
                slot_index,
                &mut remaining,
                self.inputs.start(),
                self.result.start(),
                FillDirection::Forward,
            )
        } else if self.player.main().contains(slot_index) {
            behavior.move_item_stack_to(
                guard,
                slot_index,
                &mut remaining,
                self.player.hotbar().start(),
                self.player.hotbar().end(),
                FillDirection::Forward,
            )
        } else if self.player.hotbar().contains(slot_index) {
            behavior.move_item_stack_to(
                guard,
                slot_index,
                &mut remaining,
                self.player.main().start(),
                self.player.main().end(),
                FillDirection::Forward,
            )
        } else {
            false
        };

        if !moved {
            return Some(ItemStack::empty());
        }
        behavior.update_quick_move_source(guard, slot_index, &remaining, &clicked);
        if remaining.count() == clicked.count() {
            return Some(ItemStack::empty());
        }
        if let Some(remainder) = behavior.slots()[slot_index].on_take(guard, &remaining, player) {
            player.add_item_or_drop_with_guard(guard, remainder, Prediction::Predicted);
        }
        Some(clicked)
    }
}
