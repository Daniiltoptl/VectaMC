//! Chest menu for chest-like containers (chests, barrels, ender chests, shulker boxes).
//!
//! 1-6 rows of 9 slots. Layout:
//! - Slots 0 to `rows * 9 - 1`: Container
//! - Slots `rows * 9` to `rows * 9 + 26`: Main inventory (27)
//! - Slots `rows * 9 + 27` to `rows * 9 + 35`: Hotbar (9)

use vecta_registry::menu_type::MenuTypeRef;
use vecta_registry::vanilla_menu_types;

use crate::inventory::prelude::*;
use crate::player::player_inventory::PlayerInventory;

/// Builds a chest-like menu with `rows` rows of 9 slots plus the player inventory.
///
/// # Panics
/// Panics if `rows` is 0 or greater than 6.
#[must_use]
pub fn chest(
    inventory: Shared<PlayerInventory>,
    container_id: u8,
    container: impl Into<ContainerRef>,
    rows: usize,
) -> Menu {
    let container = container.into();
    chest_with_kind(
        inventory,
        container_id,
        container.clone(),
        rows,
        ChestKind::new(container),
    )
}

impl ChestKind {
    /// Creates a chest kind for `container`.
    #[must_use]
    pub fn new(container: ContainerRef) -> Self {
        Self {
            containers: vec![container],
            on_close: None,
        }
    }

    /// Creates a chest kind for a container split across `halves`, which are
    /// listed in menu slot order.
    #[must_use]
    pub fn halves(halves: impl IntoIterator<Item = ContainerRef>) -> Self {
        Self {
            containers: halves.into_iter().collect(),
            on_close: None,
        }
    }

    /// Adds a callback that runs when the menu closes.
    #[must_use]
    pub fn with_close_hook(mut self, hook: impl Fn(&Player) + Send + Sync + 'static) -> Self {
        self.on_close = Some(Box::new(hook));
        self
    }
}

/// Builds the same layout as [`chest`] under a caller-supplied menu kind.
///
/// Chest-shaped menus whose validity or close cleanup is not the backing
/// container's own (the ender chest, whose container belongs to the player
/// rather than the block) share the layout through this.
///
/// # Panics
/// Panics if `rows` is 0 or greater than 6.
#[must_use]
pub(crate) fn chest_with_kind(
    inventory: Shared<PlayerInventory>,
    container_id: u8,
    container: impl Into<ContainerRef>,
    rows: usize,
    kind: impl MenuKind + 'static,
) -> Menu {
    let container = container.into();
    chest_parts_with_kind(inventory, container_id, &[container], rows, kind)
}

/// Builds the chest layout over `parts`, which are listed in menu slot order.
///
/// Vanilla backs a double chest with one `CompoundContainer` whose slot space is
/// the two halves concatenated. Vecta keeps every slot backed by the container
/// that stores it, so the parts become adjacent sections and one span covers the
/// logical container for shift-click.
///
/// # Panics
/// Panics if `rows` is 0 or greater than 6, if `parts` is empty, or if the
/// `rows * 9` menu slots do not split evenly across `parts`.
#[must_use]
pub(crate) fn chest_parts_with_kind(
    inventory: Shared<PlayerInventory>,
    container_id: u8,
    parts: &[ContainerRef],
    rows: usize,
    kind: impl MenuKind + 'static,
) -> Menu {
    assert!(
        (1..=6).contains(&rows),
        "Chest rows must be between 1 and 6"
    );
    assert!(!parts.is_empty(), "a chest menu needs a container");
    let slots = rows * 9;
    assert_eq!(
        slots % parts.len(),
        0,
        "a {slots}-slot chest menu cannot be split across {} containers",
        parts.len()
    );
    let slots_per_part = slots / parts.len();

    let mut builder = MenuBuilder::new(menu_type_for_rows(rows), container_id);
    let sections: Vec<Section> = parts
        .iter()
        .map(|part| builder.section(part, slots_per_part))
        .collect();
    let chest = builder.span(&sections);
    let player = builder.player_inventory(&inventory);

    builder.route(chest, player.all(), FillDirection::Backward);
    builder.route(player.all(), chest, FillDirection::Forward);

    builder.build(kind)
}

/// Menu type for a chest of `rows` rows.
///
/// # Panics
/// Panics if `rows` is 0 or greater than 6.
#[must_use]
pub fn menu_type_for_rows(rows: usize) -> MenuTypeRef {
    match rows {
        1 => &vanilla_menu_types::GENERIC_9X1,
        2 => &vanilla_menu_types::GENERIC_9X2,
        3 => &vanilla_menu_types::GENERIC_9X3,
        4 => &vanilla_menu_types::GENERIC_9X4,
        5 => &vanilla_menu_types::GENERIC_9X5,
        6 => &vanilla_menu_types::GENERIC_9X6,
        _ => panic!("Invalid row count: {rows}"),
    }
}

/// Callback invoked once when a chest-like menu closes.
pub type ChestCloseHook = Box<dyn Fn(&Player) + Send + Sync>;

/// Per-menu chest state: the backing containers plus an optional close hook.
pub struct ChestKind {
    /// The backing containers, in menu slot order. A single chest has one; a
    /// double chest has both halves.
    containers: Vec<ContainerRef>,
    /// Called once when the menu closes, after the player inventory is back.
    on_close: Option<ChestCloseHook>,
}

// SAFETY: This Vecta-owned key uniquely identifies the concrete menu kind
// within the process.
unsafe impl vecta_utils::DowncastType for ChestKind {
    const TYPE_KEY: vecta_utils::DowncastTypeKey =
        vecta_utils::DowncastTypeKey::new("vecta:menu/chest");
}

impl MenuKind for ChestKind {
    fn owns_container(&self, container: &ContainerRef) -> bool {
        let id = container.container_id();
        self.containers
            .iter()
            .any(|owned| owned.container_id() == id)
    }

    /// Returns true if every backing container is still valid for the player.
    fn still_valid(&self, _behavior: &MenuBehavior, player: &Player) -> bool {
        self.containers
            .iter()
            .all(|container| container.still_valid(player))
    }

    fn removed(&mut self, _behavior: &mut MenuBehavior, player: &Player) {
        if let Some(on_close) = &self.on_close {
            on_close(player);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use vecta_registry::vanilla_items;
    use vecta_utils::locks::IntoShared as _;

    use super::*;
    use crate::bootstrap::init_globals;
    use crate::inventory::click::Click;
    use crate::inventory::container::SimpleContainer;
    use crate::test_support::{TestPlayerBuilder, fresh_test_world};

    #[test]
    fn chest_uses_exactly_the_rows_requested_from_oversized_container() {
        let inventory = PlayerInventory::new().into_shared();
        let container = SimpleContainer::new(18).into_shared();

        let menu = chest(inventory, 1, container, 1);

        assert_eq!(menu.behavior().slot_count(), 9 + 36);
    }

    #[test]
    fn a_shift_click_fills_the_first_half_then_spills_into_the_second() {
        init_globals();
        let fixture = fresh_test_world("chest_double_quick_move");
        let player =
            TestPlayerBuilder::new(Arc::clone(&fixture.world), "DoubleQuickMove", 1).build();

        let first = SimpleContainer::new(27).into_shared();
        let second = SimpleContainer::new(27).into_shared();
        let first_ref = ContainerRef::from(Arc::clone(&first));
        let second_ref = ContainerRef::from(Arc::clone(&second));
        // The first half holds a half-full stone stack and no empty slot, so a
        // 64-stack tops it up and what is left has to cross into the second
        // half, exactly as vanilla's single `moveItemStackTo(0, 54)` range does.
        {
            let mut first = first.lock();
            first.set_item(0, ItemStack::with_count(&vanilla_items::STONE, 32));
            for slot in 1..27 {
                first.set_item(slot, ItemStack::with_count(&vanilla_items::DIRT, 64));
            }
        }
        player
            .inventory
            .lock()
            .set_item(9, ItemStack::with_count(&vanilla_items::STONE, 64));

        let mut menu = chest_parts_with_kind(
            Arc::clone(&player.inventory),
            1,
            &[first_ref.clone(), second_ref.clone()],
            6,
            ChestKind::halves([first_ref, second_ref]),
        );
        assert_eq!(menu.behavior().slot_count(), 54 + 36);

        // Slot 54 is the first player main-inventory slot of a six-row chest.
        menu.clicked(Click::QuickMove { slot: 54 }, &player)
            .expect("a shift-click on a player slot is valid");

        assert_eq!(
            first.lock().get_item(0).count(),
            64,
            "the stack tops up the existing stack in the first half"
        );
        assert_eq!(
            second.lock().get_item(0).count(),
            32,
            "what is left over spills across the half boundary"
        );
    }
}
