//! Hopper menu.
//!
//! Five hopper slots over the player inventory, matching vanilla `HopperMenu`:
//! shift-clicking a hopper slot fills the player inventory backwards, and
//! shift-clicking a player slot fills the hopper forwards.

use vecta_registry::vanilla_menu_types;

use crate::inventory::prelude::*;
use crate::player::player_inventory::PlayerInventory;

use super::chest_menu::ChestKind;

/// Vanilla `HopperMenu.CONTAINER_SIZE`.
pub const HOPPER_MENU_SLOTS: usize = 5;

/// Builds the five-slot vanilla hopper menu over `container`.
#[must_use]
pub fn hopper_menu(
    inventory: Shared<PlayerInventory>,
    container_id: u8,
    container: ContainerRef,
) -> Menu {
    let mut builder = MenuBuilder::new(&vanilla_menu_types::HOPPER, container_id);

    let hopper = builder.section(container.clone(), HOPPER_MENU_SLOTS);
    let player = builder.player_inventory(&inventory);

    builder.route(hopper, player.all(), FillDirection::Backward);
    builder.route(player.all(), hopper, FillDirection::Forward);

    // `HopperMenu.stillValid` delegates to the container, and the hopper has no
    // viewer counting or close hook, so the plain container-backed kind fits.
    builder.build(ChestKind::new(container))
}

#[cfg(test)]
mod tests {
    use vecta_registry::{init_vanilla_registry, vanilla_items};
    use vecta_utils::locks::IntoShared as _;

    use super::*;
    use crate::inventory::click::Click;
    use crate::inventory::container::SimpleContainer;
    use crate::test_support::{TestPlayerBuilder, fresh_test_world};
    use std::sync::Arc;

    #[test]
    fn the_menu_has_five_container_slots_and_the_player_inventory() {
        init_vanilla_registry();
        let inventory = PlayerInventory::new().into_shared();
        let container = ContainerRef::from(SimpleContainer::new(HOPPER_MENU_SLOTS).into_shared());

        let menu = hopper_menu(inventory, 1, container);

        assert_eq!(menu.behavior().slot_count(), HOPPER_MENU_SLOTS + 36);
    }

    #[test]
    fn a_shift_click_from_the_player_fills_the_hopper_forwards() {
        init_vanilla_registry();
        let fixture = fresh_test_world("hopper_menu_quick_move");
        let player =
            TestPlayerBuilder::new(Arc::clone(&fixture.world), "HopperQuickMove", 1).build();
        let container = SimpleContainer::new(HOPPER_MENU_SLOTS).into_shared();
        let container_ref = ContainerRef::from(Arc::clone(&container));
        player
            .inventory
            .lock()
            .set_item(9, ItemStack::with_count(&vanilla_items::STONE, 64));

        let mut menu = hopper_menu(Arc::clone(&player.inventory), 1, container_ref);

        // Slot 5 is the first player main-inventory slot of a hopper menu.
        menu.clicked(
            Click::QuickMove {
                slot: HOPPER_MENU_SLOTS,
            },
            &player,
        )
        .expect("a shift-click on a player slot is valid");

        assert_eq!(container.lock().get_item(0).count(), 64);
        assert!(
            player.inventory.lock().get_item(9).is_empty(),
            "the whole stack moved into the hopper"
        );
    }
}
