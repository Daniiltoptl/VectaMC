use std::sync::Arc;

use super::loom;
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
use vecta_registry::{
    DyeColor, REGISTRY, RegistryHolder, TaggedRegistryExt as _,
    banner_pattern::BannerPattern,
    data_components::components::{BannerPatternLayer, BannerPatternLayers},
    data_components::vanilla_components::BANNER_PATTERNS,
    init_vanilla_registry,
    item_stack::ItemStack,
    vanilla_banner_pattern_tags::BannerPatternTag,
    vanilla_banner_patterns, vanilla_blocks, vanilla_items,
};
use vecta_utils::types::UpdateFlags;
use vecta_utils::{BlockPos, ChunkPos};

const BANNER_SLOT: usize = 0;
const DYE_SLOT: usize = 1;
const PATTERN_SLOT: usize = 2;
const RESULT_SLOT: usize = 3;
/// First menu slot backed by the player's main inventory (inventory index 9).
const FIRST_INVENTORY_SLOT: usize = 4;
const SELECTED_PATTERN_DATA_SLOT: usize = 0;

fn test_loom(key: &'static str) -> (TestWorld, Arc<Player>, BlockPos, Menu) {
    init_vanilla_registry();
    init_behaviors();
    let world_fixture = fresh_test_world(key);
    let world = &world_fixture.world;
    let pos = BlockPos::new(0, 64, 0);
    insert_ready_full_chunk(world, ChunkPos::from_block_pos(pos));
    assert!(world.set_block(
        pos,
        vanilla_blocks::LOOM.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    let player = TestPlayerBuilder::new(Arc::clone(world), "LoomTester", 1).build();
    player.base().set_position_local(DVec3::new(0.5, 64.0, 0.5));
    let menu = loom(Arc::clone(&player.inventory), 1, pos, world);
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

/// Moves `stack` from the player's inventory into `slot` the way a player would.
fn put(menu: &mut Menu, player: &Player, slot: usize, stack: ItemStack) {
    player.inventory.lock().set_item(9, stack);
    click(menu, player, FIRST_INVENTORY_SLOT);
    click(menu, player, slot);
}

fn selected_pattern(menu: &Menu) -> i16 {
    menu.behavior()
        .get_data(SELECTED_PATTERN_DATA_SLOT)
        .expect("loom menu always registers its selection data slot")
}

fn no_item_required_patterns() -> Vec<&'static BannerPattern> {
    REGISTRY
        .banner_patterns
        .get_tag(&BannerPatternTag::NO_ITEM_REQUIRED)
        .expect("vanilla defines the no_item_required banner pattern tag")
}

#[test]
fn input_slots_only_accept_their_own_item_kinds() {
    let (_world, player, _pos, mut menu) = test_loom("loom_input_restrictions");
    put(
        &mut menu,
        &player,
        BANNER_SLOT,
        ItemStack::new(&vanilla_items::STONE),
    );
    assert!(slot_item(&menu, BANNER_SLOT).is_empty());

    put(
        &mut menu,
        &player,
        BANNER_SLOT,
        ItemStack::new(&vanilla_items::WHITE_BANNER),
    );
    assert_eq!(
        slot_item(&menu, BANNER_SLOT).item(),
        &*vanilla_items::WHITE_BANNER
    );

    put(
        &mut menu,
        &player,
        DYE_SLOT,
        ItemStack::new(&vanilla_items::WHITE_BANNER),
    );
    assert!(slot_item(&menu, DYE_SLOT).is_empty());
    put(
        &mut menu,
        &player,
        DYE_SLOT,
        ItemStack::new(&vanilla_items::RED_DYE),
    );
    assert_eq!(slot_item(&menu, DYE_SLOT).item(), &*vanilla_items::RED_DYE);
}

#[test]
fn plain_pattern_selection_adds_one_layer_and_take_consumes_both_inputs() {
    let (_world, player, _pos, mut menu) = test_loom("loom_plain_pattern");
    let patterns = no_item_required_patterns();
    assert!(
        patterns.len() > 1,
        "vanilla offers several patterns without a pattern item"
    );

    put(
        &mut menu,
        &player,
        BANNER_SLOT,
        ItemStack::new(&vanilla_items::WHITE_BANNER),
    );
    put(
        &mut menu,
        &player,
        DYE_SLOT,
        ItemStack::new(&vanilla_items::RED_DYE),
    );
    assert!(slot_item(&menu, RESULT_SLOT).is_empty());
    assert_eq!(selected_pattern(&menu), -1);

    assert!(menu.click_menu_button(&player, 0));
    assert_eq!(selected_pattern(&menu), 0);
    let result = slot_item(&menu, RESULT_SLOT);
    assert_eq!(result.item(), &*vanilla_items::WHITE_BANNER);
    assert_eq!(result.count(), 1);
    let layers = result
        .get(BANNER_PATTERNS)
        .expect("the loom result must carry banner layers");
    assert_eq!(layers.layers().len(), 1);
    assert_eq!(layers.layers()[0].color(), DyeColor::Red);
    assert_eq!(
        layers.layers()[0].pattern(),
        &RegistryHolder::reference(patterns[0])
    );

    click(&mut menu, &player, RESULT_SLOT);
    assert_eq!(
        menu.behavior().carried().item(),
        &*vanilla_items::WHITE_BANNER
    );
    assert!(slot_item(&menu, BANNER_SLOT).is_empty());
    assert!(slot_item(&menu, DYE_SLOT).is_empty());
    assert_eq!(selected_pattern(&menu), -1);
    assert!(slot_item(&menu, RESULT_SLOT).is_empty());
}

#[test]
fn pattern_item_replaces_the_pattern_list() {
    let (_world, player, _pos, mut menu) = test_loom("loom_pattern_item");
    put(
        &mut menu,
        &player,
        BANNER_SLOT,
        ItemStack::new(&vanilla_items::WHITE_BANNER),
    );
    put(
        &mut menu,
        &player,
        DYE_SLOT,
        ItemStack::new(&vanilla_items::BLUE_DYE),
    );
    put(
        &mut menu,
        &player,
        PATTERN_SLOT,
        ItemStack::new(&vanilla_items::CREEPER_BANNER_PATTERN),
    );

    // A single offered pattern is selected automatically.
    assert_eq!(selected_pattern(&menu), 0);
    let result = slot_item(&menu, RESULT_SLOT);
    let layers = result
        .get(BANNER_PATTERNS)
        .expect("the loom result must carry banner layers");
    assert_eq!(layers.layers().len(), 1);
    assert_eq!(
        layers.layers()[0].pattern(),
        &RegistryHolder::reference(&vanilla_banner_patterns::CREEPER)
    );
    assert_eq!(layers.layers()[0].color(), DyeColor::Blue);

    assert!(
        !menu.click_menu_button(&player, 1),
        "the pattern item offers exactly one pattern"
    );
}

/// `LoomMenu.slotsChanged` refuses to preview a seventh layer, but
/// `LoomMenu.clickMenuButton` calls `setupResultSlot` without repeating that
/// check. The result is an observable vanilla quirk: a banner that is already
/// full can still be extended by pressing a pattern button. Vecta reproduces it
/// instead of silently diverging.
#[test]
fn banner_at_six_layers_previews_nothing_until_a_pattern_button_is_pressed() {
    let (_world, player, _pos, mut menu) = test_loom("loom_max_layers");
    let mut banner = ItemStack::new(&vanilla_items::WHITE_BANNER);
    banner.set(
        BANNER_PATTERNS,
        BannerPatternLayers::new(
            (0..6)
                .map(|_| {
                    BannerPatternLayer::new(
                        RegistryHolder::reference(&vanilla_banner_patterns::CREEPER),
                        DyeColor::White,
                    )
                })
                .collect(),
        ),
    );
    put(&mut menu, &player, BANNER_SLOT, banner);
    put(
        &mut menu,
        &player,
        DYE_SLOT,
        ItemStack::new(&vanilla_items::RED_DYE),
    );

    assert!(slot_item(&menu, RESULT_SLOT).is_empty());
    assert_eq!(selected_pattern(&menu), -1);

    assert!(menu.click_menu_button(&player, 0));
    let result = slot_item(&menu, RESULT_SLOT);
    let layers = result
        .get(BANNER_PATTERNS)
        .expect("the preview banner carries its pattern layers");
    assert_eq!(
        layers.layers().len(),
        7,
        "clickMenuButton is not capped by slotsChanged in vanilla"
    );

    assert!(!menu.click_menu_button(&player, -1));
    assert!(!menu.click_menu_button(&player, 999));
}

#[test]
fn quick_move_moves_the_result_into_the_inventory() {
    let (_world, player, _pos, mut menu) = test_loom("loom_quick_move");
    put(
        &mut menu,
        &player,
        BANNER_SLOT,
        ItemStack::new(&vanilla_items::WHITE_BANNER),
    );
    put(
        &mut menu,
        &player,
        DYE_SLOT,
        ItemStack::new(&vanilla_items::RED_DYE),
    );
    assert!(menu.click_menu_button(&player, 0));
    let result = slot_item(&menu, RESULT_SLOT);

    menu.clicked(Click::QuickMove { slot: RESULT_SLOT }, &player)
        .expect("quick move should be handled");

    assert!(slot_item(&menu, BANNER_SLOT).is_empty());
    assert!(slot_item(&menu, DYE_SLOT).is_empty());
    assert!(slot_item(&menu, RESULT_SLOT).is_empty());
    let moved = player
        .inventory
        .lock()
        .items()
        .iter()
        .filter(|item| item.item() == result.item())
        .count();
    assert_eq!(moved, 1);
}
