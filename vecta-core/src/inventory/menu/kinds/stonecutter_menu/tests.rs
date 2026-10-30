use std::sync::Arc;

use super::{StonecutterKind, stonecutter};
use crate::{
    behavior::init_behaviors,
    entity::Entity as _,
    inventory::{
        click::{Click, MouseButton},
        container::Container as _,
        menu::Menu,
    },
    player::Player,
    test_support::{TestPlayerBuilder, TestWorld, fresh_test_world, insert_ready_full_chunk},
};
use glam::DVec3;
use text_components::TextComponent;
use vecta_protocol::packets::game::SContainerButtonClick;
use vecta_registry::{init_vanilla_registry, item_stack::ItemStack, vanilla_blocks, vanilla_items};
use vecta_utils::Downcast as _;
use vecta_utils::translations;
use vecta_utils::types::UpdateFlags;
use vecta_utils::{BlockPos, ChunkPos};

/// First menu slot backed by the player's main inventory (inventory index 9).
const FIRST_INVENTORY_SLOT: usize = 2;
const INPUT_SLOT: usize = 0;
const RESULT_SLOT: usize = 1;
const SELECTED_RECIPE_DATA_SLOT: usize = 0;

fn test_stonecutter(key: &'static str) -> (TestWorld, Arc<Player>, BlockPos, Menu) {
    init_vanilla_registry();
    init_behaviors();
    let world_fixture = fresh_test_world(key);
    let world = &world_fixture.world;
    let pos = BlockPos::new(0, 64, 0);
    insert_ready_full_chunk(world, ChunkPos::from_block_pos(pos));
    assert!(world.set_block(
        pos,
        vanilla_blocks::STONECUTTER.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    let player = TestPlayerBuilder::new(Arc::clone(world), "StonecutTest", 1).build();
    player.base().set_position_local(DVec3::new(0.5, 64.0, 0.5));
    let menu = stonecutter(Arc::clone(&player.inventory), 1, pos, world);
    (world_fixture, player, pos, menu)
}

fn slot_item(menu: &Menu, index: usize) -> ItemStack {
    let guard = menu.behavior().lock_all_containers();
    menu.behavior().slots()[index].get_item(&guard).clone()
}

fn click(menu: &mut Menu, player: &Player, slot: usize) {
    menu.clicked(
        Click::Pickup {
            slot,
            button: MouseButton::Left,
        },
        player,
    )
    .expect("pickup click should be handled");
}

/// Puts `stack` into the stonecutter input slot the way a player would.
fn put_input(menu: &mut Menu, player: &Player, stack: ItemStack) {
    player.inventory.lock().set_item(9, stack);
    click(menu, player, FIRST_INVENTORY_SLOT);
    click(menu, player, INPUT_SLOT);
}

fn selected_recipe(menu: &Menu) -> i16 {
    menu.behavior()
        .get_data(SELECTED_RECIPE_DATA_SLOT)
        .expect("stonecutter menu always registers its selection data slot")
}

fn take_result(menu: &mut Menu, player: &Player) -> ItemStack {
    click(menu, player, RESULT_SLOT);
    menu.behavior().carried().clone()
}

#[test]
fn selecting_two_recipes_yields_their_own_results_and_consumes_one_input() {
    let (_world, player, _pos, mut menu) = test_stonecutter("stonecutter_select_recipes");
    put_input(
        &mut menu,
        &player,
        ItemStack::with_count(&vanilla_items::STONE, 3),
    );

    assert!(slot_item(&menu, RESULT_SLOT).is_empty());
    assert_eq!(selected_recipe(&menu), -1);

    assert!(menu.click_menu_button(&player, 0));
    let first = slot_item(&menu, RESULT_SLOT);
    assert!(!first.is_empty(), "selecting a recipe must fill the result");
    assert_eq!(selected_recipe(&menu), 0);

    let taken = take_result(&mut menu, &player);
    assert_eq!(taken.item(), first.item());
    assert_eq!(slot_item(&menu, INPUT_SLOT).count(), 2);
    assert_eq!(slot_item(&menu, RESULT_SLOT).item(), first.item());

    assert!(menu.click_menu_button(&player, 1));
    let second = slot_item(&menu, RESULT_SLOT);
    assert!(!second.is_empty());
    assert_ne!(
        second.item(),
        first.item(),
        "a different button must select a different recipe"
    );
    assert_eq!(selected_recipe(&menu), 1);
}

#[test]
fn changing_the_input_item_clears_the_selection_and_result() {
    let (_world, player, _pos, mut menu) = test_stonecutter("stonecutter_input_reset");
    put_input(
        &mut menu,
        &player,
        ItemStack::with_count(&vanilla_items::STONE, 2),
    );
    assert!(menu.click_menu_button(&player, 0));
    let stone_result = slot_item(&menu, RESULT_SLOT);
    assert!(!stone_result.is_empty());

    // Swap the input for cobblestone: pick the stone up, swap with cobblestone
    // in the inventory, then place the cobblestone in the input slot.
    click(&mut menu, &player, INPUT_SLOT);
    player
        .inventory
        .lock()
        .set_item(10, ItemStack::new(&vanilla_items::COBBLESTONE));
    click(&mut menu, &player, FIRST_INVENTORY_SLOT + 1);
    click(&mut menu, &player, INPUT_SLOT);

    assert_eq!(
        slot_item(&menu, INPUT_SLOT).item(),
        &*vanilla_items::COBBLESTONE
    );
    assert_eq!(selected_recipe(&menu), -1);
    assert!(
        slot_item(&menu, RESULT_SLOT).is_empty(),
        "the stale recipe must not survive an input change"
    );

    assert!(menu.click_menu_button(&player, 0));
    let cobblestone_result = slot_item(&menu, RESULT_SLOT);
    assert!(!cobblestone_result.is_empty());
    assert_ne!(cobblestone_result.item(), stone_result.item());
}

#[test]
fn out_of_range_button_is_accepted_but_changes_nothing() {
    let (_world, player, _pos, mut menu) = test_stonecutter("stonecutter_invalid_button");
    put_input(
        &mut menu,
        &player,
        ItemStack::with_count(&vanilla_items::STONE, 2),
    );
    assert!(menu.click_menu_button(&player, 0));
    let result = slot_item(&menu, RESULT_SLOT);

    assert!(menu.click_menu_button(&player, 500));
    assert_eq!(selected_recipe(&menu), 0);
    assert_eq!(slot_item(&menu, RESULT_SLOT).item(), result.item());

    assert!(
        !menu.click_menu_button(&player, 0),
        "re-selecting the current recipe is not an accepted click"
    );
}

#[test]
fn quick_move_crafts_until_the_input_runs_out() {
    let (_world, player, _pos, mut menu) = test_stonecutter("stonecutter_quick_move");
    put_input(
        &mut menu,
        &player,
        ItemStack::with_count(&vanilla_items::STONE, 2),
    );
    assert!(menu.click_menu_button(&player, 0));
    let result = slot_item(&menu, RESULT_SLOT);

    menu.clicked(Click::QuickMove { slot: RESULT_SLOT }, &player)
        .expect("quick move should be handled");

    assert!(slot_item(&menu, INPUT_SLOT).is_empty());
    assert!(slot_item(&menu, RESULT_SLOT).is_empty());
    let moved = player
        .inventory
        .lock()
        .items()
        .iter()
        .filter(|item| item.item() == result.item())
        .map(ItemStack::count)
        .sum::<i32>();
    assert_eq!(moved, result.count() * 2);
}

#[test]
fn quick_move_drops_the_result_that_does_not_fit_and_consumes_only_one_input() {
    use crate::entity::entities::ItemEntity;
    use vecta_registry::REGISTRY;
    use vecta_registry::recipe::{SingleItemRecipeInput, vanilla_recipe_types};

    let (world_fixture, player, _pos, mut menu) = test_stonecutter("stonecutter_partial_result");
    put_input(
        &mut menu,
        &player,
        ItemStack::with_count(&vanilla_items::STONE, 3),
    );
    let index = REGISTRY
        .recipes
        .matching(
            &vanilla_recipe_types::STONECUTTING,
            &SingleItemRecipeInput::new(ItemStack::new(&vanilla_items::STONE)),
        )
        .position(|recipe| recipe.data().result.count() > 1)
        .expect("stone has a recipe producing multiple items");
    assert!(menu.click_menu_button(
        &player,
        i32::try_from(index).expect("recipe index fits i32")
    ));
    let result = slot_item(&menu, RESULT_SLOT);
    let max_count = result.max_stack_size();
    {
        let mut inventory = player.inventory.lock();
        for slot in 0..36 {
            inventory.set_item(slot, ItemStack::with_count(&vanilla_items::DIRT, 64));
        }
        inventory.set_item(8, result.copy_with_count(max_count - 1));
    }

    menu.clicked(Click::QuickMove { slot: RESULT_SLOT }, &player)
        .expect("quick move should be handled");

    assert_eq!(player.inventory.lock().get_item(8).count(), max_count);
    assert_eq!(slot_item(&menu, INPUT_SLOT).count(), 2);
    assert_eq!(slot_item(&menu, RESULT_SLOT).count(), result.count());
    let drops = world_fixture
        .world
        .get_entities_in_aabb_matching(&player.bounding_box().inflate(4.0), |entity| {
            entity.downcast_ref::<ItemEntity>().is_some()
        });
    assert_eq!(drops.len(), 1);
    let dropped = drops[0]
        .downcast_ref::<ItemEntity>()
        .expect("result was dropped as an item")
        .get_item();
    assert_eq!(dropped.item(), result.item());
    assert_eq!(dropped.count(), result.count() - 1);
}

#[test]
fn button_packet_requires_the_current_container_id() {
    init_vanilla_registry();
    init_behaviors();
    let world_fixture = fresh_test_world("stonecutter_button_packet");
    let world = &world_fixture.world;
    let pos = BlockPos::new(0, 64, 0);
    insert_ready_full_chunk(world, ChunkPos::from_block_pos(pos));
    assert!(world.set_block(
        pos,
        vanilla_blocks::STONECUTTER.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    let player = TestPlayerBuilder::new(Arc::clone(world), "StonecutPkt", 1).build();
    player.base().set_position_local(DVec3::new(0.5, 64.0, 0.5));

    let inventory = Arc::clone(&player.inventory);
    player.open_menu(
        TextComponent::translated(translations::CONTAINER_STONECUTTER.msg()),
        move |context| stonecutter(inventory, context.container_id, pos, context.world),
    );
    let container_id = player
        .with_open_menu(|menu| menu.container_id())
        .expect("the stonecutter menu must be open");
    assert!(
        player
            .with_open_menu(|menu| {
                put_input(
                    menu,
                    &player,
                    ItemStack::with_count(&vanilla_items::STONE, 2),
                );
                slot_item(menu, RESULT_SLOT).is_empty()
            })
            .expect("the stonecutter menu must be open")
    );

    player.handle_container_button_click(SContainerButtonClick {
        container_id: i32::from(container_id) + 1,
        button_id: 0,
    });
    assert!(
        player
            .with_open_menu(|menu| slot_item(menu, RESULT_SLOT).is_empty())
            .expect("the stonecutter menu must stay open")
    );

    player.handle_container_button_click(SContainerButtonClick {
        container_id: i32::from(container_id),
        button_id: 0,
    });
    assert!(
        !player
            .with_open_menu(|menu| slot_item(menu, RESULT_SLOT).is_empty())
            .expect("the stonecutter menu must stay open")
    );
}

#[test]
fn removed_block_rejects_button_clicks() {
    let (world_fixture, player, pos, mut menu) = test_stonecutter("stonecutter_removed_block");
    put_input(
        &mut menu,
        &player,
        ItemStack::with_count(&vanilla_items::STONE, 2),
    );
    assert!(menu.still_valid(&player));

    let world = &world_fixture.world;
    assert!(world.set_block(
        pos,
        vanilla_blocks::AIR.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    assert!(!menu.still_valid(&player));

    let inventory = Arc::clone(&player.inventory);
    player.open_menu(
        TextComponent::translated(translations::CONTAINER_STONECUTTER.msg()),
        move |context| stonecutter(inventory, context.container_id, pos, context.world),
    );
    let container_id = player
        .with_open_menu(|menu| menu.container_id())
        .expect("the stonecutter menu must be open");
    assert!(
        player
            .with_open_menu(|menu| {
                put_input(
                    menu,
                    &player,
                    ItemStack::with_count(&vanilla_items::STONE, 2),
                );
                slot_item(menu, RESULT_SLOT).is_empty()
            })
            .expect("the stonecutter menu must be open")
    );

    player.handle_container_button_click(SContainerButtonClick {
        container_id: i32::from(container_id),
        button_id: 0,
    });
    assert!(
        player
            .with_open_menu(|menu| slot_item(menu, RESULT_SLOT).is_empty())
            .expect("the stonecutter menu must stay open")
    );
}

#[test]
fn menu_kind_is_the_stonecutter_kind() {
    let (_world, _player, _pos, menu) = test_stonecutter("stonecutter_kind");
    assert!(menu.kind().is::<StonecutterKind>());
}
