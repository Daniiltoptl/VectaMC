//! Stonecutter menu.
//!
//! Vanilla's `StonecutterMenu` keeps one input slot, a recipe list filtered by
//! the input item, a data slot with the selected index, and a virtual result
//! slot. The client selects a recipe with a container button click, which this
//! menu validates against its own recipe list.

use std::sync::Arc;

use vecta_registry::blocks::block_state_ext::BlockStateExt;
use vecta_registry::vanilla_blocks;
use vecta_registry::vanilla_menu_types;
use vecta_utils::locks::{IntoShared as _, Shared};
use vecta_utils::{BlockPos, DowncastType, DowncastTypeKey};

use crate::{
    inventory::{
        container::{ResultContainer, SimpleContainer},
        prelude::*,
        slots::StonecutterResultHandler,
        stonecutter::{StonecutterSelection, write_result},
    },
    player::player_inventory::{PlayerInventory, Prediction},
    world::World,
};

#[cfg(test)]
mod tests;

/// Builds the stonecutter menu.
#[must_use]
pub fn stonecutter(
    inventory: Shared<PlayerInventory>,
    container_id: u8,
    pos: BlockPos,
    world: &Arc<World>,
) -> Menu {
    let input_container = SimpleContainer::new(1).into_shared();
    let result_container = ResultContainer::new().into_shared();
    let selection = StonecutterSelection::new().into_shared();

    let mut builder = MenuBuilder::new(&vanilla_menu_types::STONECUTTER, container_id);

    let input = builder.section_all(&input_container);
    let result = builder.result_slot_with(
        StonecutterResultHandler::new(
            Arc::clone(&input_container),
            Arc::clone(&result_container),
            Arc::clone(&selection),
            pos,
            Arc::clone(world),
        ),
        CraftedStat::TakenStack,
    );

    let player = builder.player_inventory(&inventory);

    builder.route_with_remainder_policy(
        result,
        player.all(),
        FillDirection::Backward,
        FakeResultRemainderPolicy::Discard,
    );
    builder.route(input, player.all(), FillDirection::Forward);
    builder.drain(input);

    let selected_recipe_index = builder.data_slot(0);

    builder.build(StonecutterKind {
        input_container,
        result_container,
        selection,
        recipes_input: ItemStack::empty(),
        block_pos: pos,
        world: Arc::clone(world),
        input,
        result,
        player,
        selected_recipe_index,
    })
}

/// Per-menu stonecutter state: the input, the offered recipes for that input,
/// and the selected recipe index.
pub struct StonecutterKind {
    input_container: Shared<SimpleContainer>,
    result_container: Shared<ResultContainer>,
    selection: Shared<StonecutterSelection>,
    /// The input stack the recipe list was built for.
    recipes_input: ItemStack,
    block_pos: BlockPos,
    world: Arc<World>,
    input: Section,
    result: Section,
    player: PlayerInventorySections,
    selected_recipe_index: DataSlot,
}

// SAFETY: This Vecta-owned key uniquely identifies the concrete menu kind
// within the process.
unsafe impl DowncastType for StonecutterKind {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:menu/stonecutter");
}

impl MenuKind for StonecutterKind {
    /// Returns true while the original stonecutter remains in range.
    fn still_valid(&self, _behavior: &MenuBehavior, player: &Player) -> bool {
        let state = self.world.get_block_state(self.block_pos);
        state.get_block() == &vanilla_blocks::STONECUTTER
            && player.is_within_block_interaction_range_with_buffer(self.block_pos, 4.0)
    }

    fn slots_changed(
        &mut self,
        behavior: &mut MenuBehavior,
        guard: &mut ContainerLockGuard,
        _player: &Player,
    ) {
        let input = {
            let Some(container) = guard.get(ContainerId::from_arc(&self.input_container)) else {
                log::warn!("input container not locked while updating the stonecutter menu");
                return;
            };
            container.get_item(0).clone()
        };

        // Vanilla only rebuilds the list when the input item type changes, so
        // the selection survives count changes.
        if input.is(self.recipes_input.item()) {
            return;
        }
        self.recipes_input = input.clone();
        self.selected_recipe_index.set(behavior, -1);
        self.selection.lock().rebuild(&input);
        write_result(guard, &self.result_container, ItemStack::empty());
    }

    /// Clears the virtual result on close. The input is drained by [`Menu::removed`].
    fn removed(&mut self, _behavior: &mut MenuBehavior, _player: &Player) {
        self.result_container.lock().set_item(0, ItemStack::empty());
    }

    fn click_menu_button(
        &mut self,
        behavior: &mut MenuBehavior,
        guard: &mut ContainerLockGuard,
        _player: &Player,
        button_id: i32,
    ) -> bool {
        let mut selection = self.selection.lock();
        if selection.selected() == button_id {
            return false;
        }
        let Some(index) = selection.select(button_id) else {
            // Vanilla accepts out-of-range ids without changing the selection.
            return true;
        };
        let result = selection.result();
        drop(selection);

        self.selected_recipe_index.set(behavior, index);
        write_result(guard, &self.result_container, result);
        true
    }

    fn can_take_item_for_pick_all(&self, _carried: &ItemStack, slot_index: usize) -> bool {
        !self.result.contains(slot_index)
    }

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
        } else if self.input.contains(slot_index) {
            behavior.move_item_stack_to(
                guard,
                slot_index,
                &mut remaining,
                self.player.all().start(),
                self.player.all().end(),
                FillDirection::Forward,
            )
        } else if StonecutterSelection::accepts_input(&clicked) {
            behavior.move_item_stack_to(
                guard,
                slot_index,
                &mut remaining,
                self.input.start(),
                self.input.end(),
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
        if self.result.contains(slot_index) && !remaining.is_empty() {
            let _ =
                guard.run_unlocked(|| player.drop_item(remaining, false, Prediction::Predicted));
        }
        Some(clicked)
    }
}
