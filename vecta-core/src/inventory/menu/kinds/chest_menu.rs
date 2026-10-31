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
    pub const fn new(container: ContainerRef) -> Self {
        Self {
            container,
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
    assert!(
        (1..=6).contains(&rows),
        "Chest rows must be between 1 and 6"
    );

    let mut builder = MenuBuilder::new(menu_type_for_rows(rows), container_id);
    let chest = builder.section(&container, rows * 9);
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

/// Per-menu chest state: the backing container plus an optional close hook.
pub struct ChestKind {
    /// The backing container.
    container: ContainerRef,
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
        self.container.container_id() == container.container_id()
    }

    /// Returns true if the backing container is still valid for the player.
    fn still_valid(&self, _behavior: &MenuBehavior, player: &Player) -> bool {
        self.container.still_valid(player)
    }

    fn removed(&mut self, _behavior: &mut MenuBehavior, player: &Player) {
        if let Some(on_close) = &self.on_close {
            on_close(player);
        }
    }
}

#[cfg(test)]
mod tests {
    use vecta_utils::locks::IntoShared as _;

    use super::*;
    use crate::inventory::container::SimpleContainer;

    #[test]
    fn chest_uses_exactly_the_rows_requested_from_oversized_container() {
        let inventory = PlayerInventory::new().into_shared();
        let container = SimpleContainer::new(18).into_shared();

        let menu = chest(inventory, 1, container, 1);

        assert_eq!(menu.behavior().slot_count(), 9 + 36);
    }
}
