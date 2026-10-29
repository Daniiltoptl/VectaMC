//! Loom menu.
//!
//! Vanilla's `LoomMenu` has three inputs (banner, dye, optional pattern item),
//! a pattern list derived from the pattern item or from the
//! `banner_pattern/no_item_required` tag, a data slot with the selected pattern,
//! and a virtual result slot holding a copy of the banner with one more layer.

use std::sync::Arc;

use vecta_registry::REGISTRY;
use vecta_registry::TaggedRegistryExt as _;
use vecta_registry::banner_pattern::BannerPatternRef;
use vecta_registry::blocks::block_state_ext::BlockStateExt;
use vecta_registry::data_components::components::{BannerPatternLayer, BannerPatternLayers};
use vecta_registry::data_components::vanilla_components::{
    BANNER_PATTERNS, DYE, PROVIDES_BANNER_PATTERNS,
};
use vecta_registry::vanilla_banner_pattern_tags::BannerPatternTag;
use vecta_registry::vanilla_blocks;
use vecta_registry::vanilla_item_tags::ItemTag;
use vecta_registry::vanilla_menu_types;
use vecta_registry::{RegistryHolder, RegistryHolderSet};
use vecta_utils::locks::{IntoShared as _, Shared};
use vecta_utils::{BlockPos, DowncastType, DowncastTypeKey};

use crate::{
    inventory::{
        container::{ResultContainer, SimpleContainer},
        prelude::*,
        slots::LoomResultHandler,
    },
    player::player_inventory::{PlayerInventory, Prediction},
    world::World,
};

#[cfg(test)]
mod tests;

/// Maximum number of layers a banner can carry before the loom refuses more.
const MAX_BANNER_PATTERNS: usize = 6;

/// Builds the loom menu.
#[must_use]
pub fn loom(
    inventory: Shared<PlayerInventory>,
    container_id: u8,
    pos: BlockPos,
    world: &Arc<World>,
) -> Menu {
    let input_container = SimpleContainer::new(3).into_shared();
    let result_container = ResultContainer::new().into_shared();

    let mut builder = MenuBuilder::new(&vanilla_menu_types::LOOM, container_id);

    let inputs =
        builder.section_all_with(&input_container, SectionKind::restricted(is_allowed_input));
    let result = builder.result_slot(LoomResultHandler::new(
        Arc::clone(&input_container),
        Arc::clone(&result_container),
        pos,
        Arc::clone(world),
    ));

    let player = builder.player_inventory(&inventory);

    builder.route_with_remainder_policy(
        result,
        player.all(),
        FillDirection::Backward,
        FakeResultRemainderPolicy::Discard,
    );
    builder.route(inputs, player.all(), FillDirection::Forward);
    builder.drain(inputs);

    let selected_pattern_index = builder.data_slot(0);

    builder.build(LoomKind {
        input_container,
        result_container,
        patterns: Vec::new(),
        selected: 0,
        block_pos: pos,
        world: Arc::clone(world),
        inputs,
        result,
        player,
        selected_pattern_index,
    })
}

/// Whether `stack` may be placed in the loom's input slot `index`.
///
/// Vanilla gates each input on an item class or tag plus the component that the
/// menu actually reads: dyes need `minecraft:dye`, pattern items need
/// `minecraft:provides_banner_patterns`, and banners are the `#minecraft:banners`
/// tag (the data-driven equivalent of vanilla's `BannerItem` class).
fn is_allowed_input(index: usize, stack: &ItemStack) -> bool {
    match index {
        0 => is_banner(stack),
        1 => is_dye(stack),
        2 => is_pattern_item(stack),
        _ => false,
    }
}

fn is_banner(stack: &ItemStack) -> bool {
    REGISTRY.items.is_in_tag(stack.item(), &ItemTag::BANNERS)
}

fn is_dye(stack: &ItemStack) -> bool {
    !stack.is_empty()
        && REGISTRY.items.is_in_tag(stack.item(), &ItemTag::LOOM_DYES)
        && stack.has(DYE)
}

fn is_pattern_item(stack: &ItemStack) -> bool {
    !stack.is_empty()
        && REGISTRY
            .items
            .is_in_tag(stack.item(), &ItemTag::LOOM_PATTERNS)
        && stack.has(PROVIDES_BANNER_PATTERNS)
}

/// The patterns the loom offers for `pattern_stack`, in vanilla order.
fn selectable_patterns(pattern_stack: &ItemStack) -> Vec<BannerPatternRef> {
    if pattern_stack.is_empty() {
        return REGISTRY
            .banner_patterns
            .get_tag(&BannerPatternTag::NO_ITEM_REQUIRED)
            .unwrap_or_default();
    }

    pattern_stack
        .get(PROVIDES_BANNER_PATTERNS)
        .map_or_else(Vec::new, RegistryHolderSet::entries)
}

/// Per-menu loom state: inputs, offered patterns, and the selected pattern.
pub struct LoomKind {
    input_container: Shared<SimpleContainer>,
    result_container: Shared<ResultContainer>,
    patterns: Vec<BannerPatternRef>,
    selected: i32,
    block_pos: BlockPos,
    world: Arc<World>,
    inputs: Section,
    result: Section,
    player: PlayerInventorySections,
    selected_pattern_index: DataSlot,
}

// SAFETY: This Vecta-owned key uniquely identifies the concrete menu kind
// within the process.
unsafe impl DowncastType for LoomKind {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:menu/loom");
}

impl LoomKind {
    fn is_valid_pattern_index(&self, index: i32) -> bool {
        usize::try_from(index).is_ok_and(|index| index < self.patterns.len())
    }

    /// Writes the pattern button selection to the synced data slot.
    fn publish_selection(&self, behavior: &mut MenuBehavior) {
        let value = i16::try_from(self.selected).unwrap_or(-1);
        self.selected_pattern_index.set(behavior, value);
    }

    /// Builds the result for `pattern` from the current banner and dye, matching
    /// `LoomMenu.setupResultSlot`.
    fn setup_result(&self, guard: &mut ContainerLockGuard, pattern: BannerPatternRef) {
        let Some(inputs) = guard.get(ContainerId::from_arc(&self.input_container)) else {
            log::warn!("input container not locked while updating the loom result");
            return;
        };
        let banner = inputs.get_item(0);
        let dye = inputs.get_item(1);
        let mut result = ItemStack::empty();
        if !banner.is_empty()
            && !dye.is_empty()
            && let Some(color) = dye.get(DYE).copied()
        {
            let mut layers = banner
                .get(BANNER_PATTERNS)
                .map_or_else(Vec::new, |layers| layers.layers().to_vec());
            layers.push(BannerPatternLayer::new(
                RegistryHolder::reference(pattern),
                color,
            ));
            result = banner.copy_with_count(1);
            result.set(BANNER_PATTERNS, BannerPatternLayers::new(layers));
        }

        let current = guard
            .get(ContainerId::from_arc(&self.result_container))
            .map_or_else(ItemStack::empty, |container| container.get_item(0).clone());
        if ItemStack::matches(&result, &current) {
            return;
        }
        let Some(result_container) = guard.get_mut(ContainerId::from_arc(&self.result_container))
        else {
            log::warn!("result container not locked while updating the loom result");
            return;
        };
        result_container.set_item(0, result);
    }

    fn clear_result(&self, guard: &mut ContainerLockGuard) {
        let Some(result_container) = guard.get_mut(ContainerId::from_arc(&self.result_container))
        else {
            log::warn!("result container not locked while clearing the loom result");
            return;
        };
        result_container.set_item(0, ItemStack::empty());
    }
}

impl MenuKind for LoomKind {
    /// Returns true while the original loom remains in range.
    fn still_valid(&self, _behavior: &MenuBehavior, player: &Player) -> bool {
        let state = self.world.get_block_state(self.block_pos);
        state.get_block() == &vanilla_blocks::LOOM
            && player.is_within_block_interaction_range_with_buffer(self.block_pos, 4.0)
    }

    fn slots_changed(
        &mut self,
        behavior: &mut MenuBehavior,
        guard: &mut ContainerLockGuard,
        _player: &Player,
    ) {
        let (banner, dye, pattern_item) = {
            let Some(inputs) = guard.get(ContainerId::from_arc(&self.input_container)) else {
                log::warn!("input container not locked while updating the loom menu");
                return;
            };
            (
                inputs.get_item(0).clone(),
                inputs.get_item(1).clone(),
                inputs.get_item(2).clone(),
            )
        };

        if banner.is_empty() || dye.is_empty() {
            self.clear_result(guard);
            self.patterns.clear();
            self.selected = -1;
            self.publish_selection(behavior);
            return;
        }

        let previous_selection = if self.is_valid_pattern_index(self.selected) {
            self.patterns
                .get(usize::try_from(self.selected).unwrap_or(0))
                .copied()
        } else {
            None
        };
        self.patterns = selectable_patterns(&pattern_item);

        let displayed = if self.patterns.len() == 1 {
            self.selected = 0;
            self.patterns.first().copied()
        } else if let Some(selected) = previous_selection {
            if let Some(index) = self
                .patterns
                .iter()
                .position(|pattern| *pattern == selected)
            {
                self.selected = i32::try_from(index).unwrap_or(-1);
                Some(selected)
            } else {
                self.selected = -1;
                None
            }
        } else {
            self.selected = -1;
            None
        };

        let Some(pattern) = displayed else {
            self.clear_result(guard);
            self.publish_selection(behavior);
            return;
        };

        let layer_count = banner
            .get(BANNER_PATTERNS)
            .map_or(0, |layers| layers.layers().len());
        if layer_count >= MAX_BANNER_PATTERNS {
            self.selected = -1;
            self.clear_result(guard);
        } else {
            self.setup_result(guard, pattern);
        }
        self.publish_selection(behavior);
    }

    /// Clears the virtual result on close. The inputs are drained by [`Menu::removed`].
    fn removed(&mut self, _behavior: &mut MenuBehavior, _player: &Player) {
        self.result_container.lock().set_item(0, ItemStack::empty());
    }

    /// Mirrors `LoomMenu.clickMenuButton`.
    ///
    /// Vanilla only rejects out-of-range ids here; it calls `setupResultSlot`
    /// without repeating the six-layer cap that [`Self::slots_changed`] applies,
    /// so a banner that is already full can still gain a seventh layer through a
    /// pattern button. That is observable vanilla behaviour and is reproduced on
    /// purpose instead of being "fixed" locally.
    fn click_menu_button(
        &mut self,
        behavior: &mut MenuBehavior,
        guard: &mut ContainerLockGuard,
        _player: &Player,
        button_id: i32,
    ) -> bool {
        if !self.is_valid_pattern_index(button_id) {
            return false;
        }
        let Some(pattern) = usize::try_from(button_id)
            .ok()
            .and_then(|index| self.patterns.get(index))
            .copied()
        else {
            return false;
        };
        self.selected = button_id;
        self.publish_selection(behavior);
        self.setup_result(guard, pattern);
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
        } else if self.inputs.contains(slot_index) {
            behavior.move_item_stack_to(
                guard,
                slot_index,
                &mut remaining,
                self.player.all().start(),
                self.player.all().end(),
                FillDirection::Forward,
            )
        } else if is_banner(&clicked) {
            self.move_to_input(behavior, guard, slot_index, &mut remaining, 0)
        } else if is_dye(&clicked) {
            self.move_to_input(behavior, guard, slot_index, &mut remaining, 1)
        } else if is_pattern_item(&clicked) {
            self.move_to_input(behavior, guard, slot_index, &mut remaining, 2)
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

impl LoomKind {
    fn move_to_input(
        &self,
        behavior: &MenuBehavior,
        guard: &mut ContainerLockGuard,
        slot_index: usize,
        remaining: &mut ItemStack,
        input_index: usize,
    ) -> bool {
        let target = self.inputs.start() + input_index;
        behavior.move_item_stack_to(
            guard,
            slot_index,
            remaining,
            target,
            target + 1,
            FillDirection::Forward,
        )
    }
}
