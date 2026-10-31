//! Dispenser and dropper menu.
//!
//! Nine container slots over the player inventory, matching vanilla
//! `DispenserMenu`: shift-clicking a container slot fills the player inventory
//! backwards, and shift-clicking a player slot fills the container forwards.
//! Vanilla shares this menu between dispensers and droppers and uses the
//! `minecraft:generic_3x3` menu type.

use vecta_registry::vanilla_menu_types;

use crate::inventory::prelude::*;
use crate::player::player_inventory::PlayerInventory;

use super::chest_menu::ChestKind;

/// Vanilla `DispenserMenu.SLOT_COUNT`.
pub const DISPENSER_MENU_SLOTS: usize = 9;

/// Builds the nine-slot vanilla dispenser menu over `container`.
#[must_use]
pub fn dispenser_menu(
    inventory: Shared<PlayerInventory>,
    container_id: u8,
    container: ContainerRef,
) -> Menu {
    let mut builder = MenuBuilder::new(&vanilla_menu_types::GENERIC_3X3, container_id);

    let dispenser = builder.section(container.clone(), DISPENSER_MENU_SLOTS);
    let player = builder.player_inventory(&inventory);

    builder.route(dispenser, player.all(), FillDirection::Backward);
    builder.route(player.all(), dispenser, FillDirection::Forward);

    // `DispenserMenu.stillValid` delegates to the container, and neither the
    // dispenser nor the dropper counts viewers, so the plain kind fits.
    builder.build(ChestKind::new(container))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use vecta_registry::{init_vanilla_registry, vanilla_items};
    use vecta_utils::locks::IntoShared as _;

    use super::*;
    use crate::inventory::click::Click;
    use crate::inventory::container::SimpleContainer;
    use crate::test_support::{TestPlayerBuilder, fresh_test_world};

    #[test]
    fn the_menu_has_nine_container_slots_and_the_player_inventory() {
        init_vanilla_registry();
        let inventory = PlayerInventory::new().into_shared();
        let container =
            ContainerRef::from(SimpleContainer::new(DISPENSER_MENU_SLOTS).into_shared());

        let menu = dispenser_menu(inventory, 1, container);

        assert_eq!(menu.behavior().slot_count(), DISPENSER_MENU_SLOTS + 36);
    }

    #[test]
    fn a_shift_click_from_the_player_fills_the_container_forwards() {
        init_vanilla_registry();
        let fixture = fresh_test_world("dispenser_menu_quick_move");
        let player = TestPlayerBuilder::new(Arc::clone(&fixture.world), "DispenserMove", 1).build();
        let container = SimpleContainer::new(DISPENSER_MENU_SLOTS).into_shared();
        let container_ref = ContainerRef::from(Arc::clone(&container));
        player
            .inventory
            .lock()
            .set_item(9, ItemStack::with_count(&vanilla_items::STONE, 64));

        let mut menu = dispenser_menu(Arc::clone(&player.inventory), 1, container_ref);

        menu.clicked(
            Click::QuickMove {
                slot: DISPENSER_MENU_SLOTS,
            },
            &player,
        )
        .expect("a shift-click on a player slot is valid");

        assert_eq!(container.lock().get_item(0).count(), 64);
        assert!(
            player.inventory.lock().get_item(9).is_empty(),
            "the whole stack moved into the container"
        );
    }
}
