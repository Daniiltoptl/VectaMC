//! Lectern menu.
//!
//! Vanilla's `LecternMenu` has a single slot holding the lectern's book plus one
//! data slot with the page every reader is on. The block entity owns both, so the
//! menu mirrors the page from it on every tick and writes page changes back.

use vecta_registry::vanilla_menu_types;
use vecta_utils::{Downcast as _, DowncastType, DowncastTypeKey};

use crate::{
    block_entity::{
        SharedBlockEntity,
        entities::{LecternBlockEntity, LecternBookContainer},
    },
    inventory::{lock::ContainerRef, prelude::*},
    player::{Player, player_inventory::Prediction},
};

#[cfg(test)]
mod tests;

/// Vanilla `LecternMenu.BUTTON_PREV_PAGE`.
pub const BUTTON_PREV_PAGE: i32 = 1;
/// Vanilla `LecternMenu.BUTTON_NEXT_PAGE`.
pub const BUTTON_NEXT_PAGE: i32 = 2;
/// Vanilla `LecternMenu.BUTTON_TAKE_BOOK`.
pub const BUTTON_TAKE_BOOK: i32 = 3;
/// Vanilla `LecternMenu.BUTTON_PAGE_JUMP_RANGE_START`.
pub const BUTTON_PAGE_JUMP_RANGE_START: i32 = 100;

/// Builds the lectern menu.
///
/// The menu has no player inventory slots: vanilla's lectern screen only shows
/// the book and the page controls.
#[must_use]
pub fn lectern(container_id: u8, container: ContainerRef, block_entity: SharedBlockEntity) -> Menu {
    let initial_page = block_entity
        .downcast_ref::<LecternBlockEntity>()
        .map_or(0, LecternBlockEntity::page);

    let mut builder = MenuBuilder::new(&vanilla_menu_types::LECTERN, container_id);
    // The lectern's `bookAccess.canPlaceItem` rejects every stack, so the only
    // way the book changes is a page button or a click that takes it out.
    builder.section_all_with(container.clone(), SectionKind::restricted(|_, _| false));
    let page = builder.data_slot(i16::try_from(initial_page).unwrap_or(0));

    builder.build(LecternKind {
        container,
        block_entity,
        page,
    })
}

/// Per-menu lectern state: the book slot, the synced page and the block entity.
pub struct LecternKind {
    container: ContainerRef,
    block_entity: SharedBlockEntity,
    page: DataSlot,
}

// SAFETY: This Vecta-owned key uniquely identifies the concrete menu kind
// within the process.
unsafe impl DowncastType for LecternKind {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:menu/lectern");
}

impl LecternKind {
    fn lectern(&self) -> Option<&LecternBlockEntity> {
        self.block_entity.downcast_ref::<LecternBlockEntity>()
    }

    /// Applies a page change through the already-locked book container and
    /// mirrors the clamped result into the synced data slot.
    ///
    /// The page lives in the block entity's container, so it is read and written
    /// through the guard this hook receives; locking it again would deadlock.
    fn change_page(
        &mut self,
        behavior: &mut MenuBehavior,
        guard: &mut ContainerLockGuard,
        page: i32,
    ) {
        let id = self.container.container_id();
        let Some((page, changed)) = guard
            .get_typed_mut::<LecternBookContainer>(id)
            .map(|container| container.set_page(page))
        else {
            return;
        };
        if changed {
            guard.set_changed(id);
        }
        self.page.set(behavior, i16::try_from(page).unwrap_or(0));
    }
}

impl MenuKind for LecternKind {
    /// Vanilla's `bookAccess.stillValid` requires the block entity to survive
    /// and the slot to still hold a book, so taking the book closes the screen.
    fn still_valid(&self, _behavior: &MenuBehavior, player: &Player) -> bool {
        self.container.still_valid(player)
            && self.lectern().is_some_and(LecternBlockEntity::has_book)
    }

    /// Keeps every viewer on the page the block entity holds.
    fn on_tick(
        &mut self,
        behavior: &mut MenuBehavior,
        guard: &mut ContainerLockGuard,
        _player: &Player,
    ) {
        let id = self.container.container_id();
        let Some(page) = guard
            .get_typed::<LecternBookContainer>(id)
            .map(LecternBookContainer::page)
        else {
            return;
        };
        let page = i16::try_from(page).unwrap_or(0);
        if self.page.get(behavior) != page {
            self.page.set(behavior, page);
        }
    }

    /// Mirrors `LecternMenu.clickMenuButton`.
    fn click_menu_button(
        &mut self,
        behavior: &mut MenuBehavior,
        guard: &mut ContainerLockGuard,
        player: &Player,
        button_id: i32,
    ) -> bool {
        if button_id >= BUTTON_PAGE_JUMP_RANGE_START {
            self.change_page(behavior, guard, button_id - BUTTON_PAGE_JUMP_RANGE_START);
            return true;
        }

        match button_id {
            BUTTON_PREV_PAGE => {
                let current = i32::from(self.page.get(behavior));
                self.change_page(behavior, guard, current - 1);
                true
            }
            BUTTON_NEXT_PAGE => {
                let current = i32::from(self.page.get(behavior));
                self.change_page(behavior, guard, current + 1);
                true
            }
            BUTTON_TAKE_BOOK => {
                if !player.abilities.lock().may_build {
                    return false;
                }
                let id = self.container.container_id();
                let book = guard
                    .get_mut(id)
                    .map_or_else(ItemStack::empty, |container| {
                        container.remove_item_no_update(0)
                    });
                guard.set_changed(id);
                // Vanilla adds the book to the inventory and drops it when it
                // does not fit; both need the container locks released first.
                guard.run_unlocked(|| player.add_item_or_drop(book, Prediction::Predicted));
                true
            }
            _ => false,
        }
    }

    /// Vanilla's lectern never shift-moves anything.
    fn quick_move(
        &mut self,
        _behavior: &mut MenuBehavior,
        _guard: &mut ContainerLockGuard,
        _slot_index: usize,
        _player: &Player,
    ) -> Option<ItemStack> {
        Some(ItemStack::empty())
    }
}
