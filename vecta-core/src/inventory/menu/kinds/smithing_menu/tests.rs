use std::sync::Arc;

use glam::DVec3;
use text_components::TextComponent;
use vecta_registry::{
    REGISTRY, RegistryExt as _, RegistryHolder,
    data_components::components::ArmorTrim,
    data_components::vanilla_components::{CUSTOM_NAME, TRIM},
    init_vanilla_registry,
    item_stack::ItemStack,
    trim_material::TrimMaterial,
    trim_pattern::TrimPattern,
    vanilla_blocks, vanilla_items,
};
use vecta_utils::types::UpdateFlags;
use vecta_utils::{BlockPos, ChunkPos, Identifier};

use super::smithing;
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

const TEMPLATE_SLOT: usize = 0;
const BASE_SLOT: usize = 1;
const ADDITION_SLOT: usize = 2;
const RESULT_SLOT: usize = 3;
/// First menu slot backed by the player's main inventory (inventory index 9).
const FIRST_INVENTORY_SLOT: usize = 4;
const HAS_RECIPE_ERROR_DATA_SLOT: usize = 0;
/// Player inventory slot the tests stage stacks in.
const STAGING_SLOT: usize = 9;
const BASE_DAMAGE: i32 = 5;
const MAIN_INVENTORY_SLOT_COUNT: usize = 36;
const FULL_STACK_COUNT: i32 = 64;

fn test_smithing(key: &'static str) -> (TestWorld, Arc<Player>, BlockPos, Menu) {
    init_vanilla_registry();
    init_behaviors();
    let world_fixture = fresh_test_world(key);
    let world = &world_fixture.world;
    let pos = BlockPos::new(0, 64, 0);
    insert_ready_full_chunk(world, ChunkPos::from_block_pos(pos));
    assert!(world.set_block(
        pos,
        vanilla_blocks::SMITHING_TABLE.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    let player = TestPlayerBuilder::new(Arc::clone(world), "SmithingTester", 1).build();
    player.base().set_position_local(DVec3::new(0.5, 64.0, 0.5));
    let menu = smithing(Arc::clone(&player.inventory), 1, pos, world);
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
    player.inventory.lock().set_item(STAGING_SLOT, stack);
    click(menu, player, FIRST_INVENTORY_SLOT);
    click(menu, player, slot);
}

fn recipe_error(menu: &Menu) -> i16 {
    menu.behavior()
        .get_data(HAS_RECIPE_ERROR_DATA_SLOT)
        .expect("smithing menu always registers its recipe-error data slot")
}

fn trim_pattern(key: &'static str) -> &'static TrimPattern {
    REGISTRY
        .trim_patterns
        .by_key(&Identifier::vanilla_static(key))
        .expect("vanilla defines this trim pattern")
}

fn trim_material(key: &'static str) -> &'static TrimMaterial {
    REGISTRY
        .trim_materials
        .by_key(&Identifier::vanilla_static(key))
        .expect("vanilla defines this trim material")
}

#[test]
fn netherite_upgrade_produces_the_result_and_keeps_the_base_components() {
    let (_world, player, _pos, mut menu) = test_smithing("smithing_upgrade");

    put(
        &mut menu,
        &player,
        TEMPLATE_SLOT,
        ItemStack::new(&vanilla_items::NETHERITE_UPGRADE_SMITHING_TEMPLATE),
    );
    let mut boots = ItemStack::new(&vanilla_items::DIAMOND_BOOTS);
    boots.set_damage_value(BASE_DAMAGE);
    boots.set(CUSTOM_NAME, TextComponent::plain("Runner"));
    put(&mut menu, &player, BASE_SLOT, boots);
    assert!(
        slot_item(&menu, RESULT_SLOT).is_empty(),
        "a missing addition must not produce a result"
    );

    put(
        &mut menu,
        &player,
        ADDITION_SLOT,
        ItemStack::new(&vanilla_items::NETHERITE_INGOT),
    );

    let result = slot_item(&menu, RESULT_SLOT);
    assert!(result.is(&vanilla_items::NETHERITE_BOOTS));
    assert_eq!(result.count(), 1);
    // `TransmuteRecipe.createWithOriginalComponents` applies the base item's
    // component patch, so damage and custom name survive the upgrade.
    assert_eq!(result.get_damage_value(), BASE_DAMAGE);
    assert_eq!(
        result.get(CUSTOM_NAME),
        Some(&TextComponent::plain("Runner")),
        "the custom name carries over through the component patch",
    );
    assert_eq!(recipe_error(&menu), 0);
}

#[test]
fn taking_the_result_consumes_one_item_from_every_input() {
    let (_world, player, _pos, mut menu) = test_smithing("smithing_take");

    // The template and the trim material are stackable; the base is a single
    // helmet, so a successful take leaves it empty.
    put(
        &mut menu,
        &player,
        TEMPLATE_SLOT,
        ItemStack::with_count(&vanilla_items::SENTRY_ARMOR_TRIM_SMITHING_TEMPLATE, 2),
    );
    put(
        &mut menu,
        &player,
        BASE_SLOT,
        ItemStack::new(&vanilla_items::IRON_HELMET),
    );
    put(
        &mut menu,
        &player,
        ADDITION_SLOT,
        ItemStack::with_count(&vanilla_items::GOLD_INGOT, 2),
    );
    assert_eq!(slot_item(&menu, TEMPLATE_SLOT).count(), 2);
    assert_eq!(slot_item(&menu, BASE_SLOT).count(), 1);
    assert_eq!(slot_item(&menu, ADDITION_SLOT).count(), 2);
    assert!(slot_item(&menu, RESULT_SLOT).is(&vanilla_items::IRON_HELMET));

    click(&mut menu, &player, RESULT_SLOT);

    assert_eq!(slot_item(&menu, TEMPLATE_SLOT).count(), 1);
    assert!(slot_item(&menu, BASE_SLOT).is_empty());
    assert_eq!(slot_item(&menu, ADDITION_SLOT).count(), 1);
    assert!(
        slot_item(&menu, RESULT_SLOT).is_empty(),
        "without a base item there is no further result"
    );
}

#[test]
fn trims_apply_the_material_and_pattern() {
    let (_world, player, _pos, mut menu) = test_smithing("smithing_trim");

    put(
        &mut menu,
        &player,
        TEMPLATE_SLOT,
        ItemStack::new(&vanilla_items::SENTRY_ARMOR_TRIM_SMITHING_TEMPLATE),
    );
    put(
        &mut menu,
        &player,
        BASE_SLOT,
        ItemStack::new(&vanilla_items::IRON_HELMET),
    );
    put(
        &mut menu,
        &player,
        ADDITION_SLOT,
        ItemStack::new(&vanilla_items::GOLD_INGOT),
    );

    let result = slot_item(&menu, RESULT_SLOT);
    assert!(result.is(&vanilla_items::IRON_HELMET));
    assert_eq!(result.count(), 1);
    let trim = result.get(TRIM).expect("a trim recipe writes the trim");
    assert_eq!(
        trim.pattern(),
        &RegistryHolder::reference(trim_pattern("sentry"))
    );
    assert_eq!(
        trim.material(),
        &RegistryHolder::reference(trim_material("gold"))
    );
}

#[test]
fn an_identical_trim_yields_no_result_but_a_different_pattern_does() {
    let (_world, player, _pos, mut menu) = test_smithing("smithing_trim_same");
    let sentry = RegistryHolder::reference(trim_pattern("sentry"));
    let coast = RegistryHolder::reference(trim_pattern("coast"));
    let gold = RegistryHolder::reference(trim_material("gold"));

    let mut already_trimmed = ItemStack::new(&vanilla_items::IRON_HELMET);
    already_trimmed.set(TRIM, ArmorTrim::new(gold.clone(), sentry.clone()));
    put(&mut menu, &player, BASE_SLOT, already_trimmed);
    put(
        &mut menu,
        &player,
        TEMPLATE_SLOT,
        ItemStack::new(&vanilla_items::SENTRY_ARMOR_TRIM_SMITHING_TEMPLATE),
    );
    put(
        &mut menu,
        &player,
        ADDITION_SLOT,
        ItemStack::new(&vanilla_items::GOLD_INGOT),
    );

    assert!(
        slot_item(&menu, RESULT_SLOT).is_empty(),
        "re-applying the same material and pattern must not produce a result"
    );
    assert_eq!(recipe_error(&menu), 1);

    let (_other_world, other_player, _pos, mut other_menu) =
        test_smithing("smithing_trim_different");
    let mut already_trimmed = ItemStack::new(&vanilla_items::IRON_HELMET);
    already_trimmed.set(TRIM, ArmorTrim::new(gold, sentry));
    put(&mut other_menu, &other_player, BASE_SLOT, already_trimmed);
    put(
        &mut other_menu,
        &other_player,
        TEMPLATE_SLOT,
        ItemStack::new(&vanilla_items::COAST_ARMOR_TRIM_SMITHING_TEMPLATE),
    );
    put(
        &mut other_menu,
        &other_player,
        ADDITION_SLOT,
        ItemStack::new(&vanilla_items::GOLD_INGOT),
    );

    let result = slot_item(&other_menu, RESULT_SLOT);
    assert!(result.is(&vanilla_items::IRON_HELMET));
    assert_eq!(
        result.get(TRIM).map(ArmorTrim::pattern),
        Some(&coast),
        "a different pattern replaces the existing trim",
    );
}

#[test]
fn mismatched_template_and_base_report_the_recipe_error() {
    let (_world, player, _pos, mut menu) = test_smithing("smithing_mismatch");

    put(
        &mut menu,
        &player,
        TEMPLATE_SLOT,
        ItemStack::new(&vanilla_items::NETHERITE_UPGRADE_SMITHING_TEMPLATE),
    );
    put(
        &mut menu,
        &player,
        BASE_SLOT,
        ItemStack::new(&vanilla_items::IRON_HELMET),
    );
    put(
        &mut menu,
        &player,
        ADDITION_SLOT,
        ItemStack::new(&vanilla_items::NETHERITE_INGOT),
    );

    assert!(slot_item(&menu, RESULT_SLOT).is_empty());
    assert_eq!(recipe_error(&menu), 1);
}

#[test]
fn replacing_the_base_before_taking_recomputes_the_result() {
    let (_world, player, _pos, mut menu) = test_smithing("smithing_input_change");

    put(
        &mut menu,
        &player,
        TEMPLATE_SLOT,
        ItemStack::new(&vanilla_items::NETHERITE_UPGRADE_SMITHING_TEMPLATE),
    );
    put(
        &mut menu,
        &player,
        BASE_SLOT,
        ItemStack::new(&vanilla_items::DIAMOND_BOOTS),
    );
    put(
        &mut menu,
        &player,
        ADDITION_SLOT,
        ItemStack::new(&vanilla_items::NETHERITE_INGOT),
    );
    assert!(slot_item(&menu, RESULT_SLOT).is(&vanilla_items::NETHERITE_BOOTS));

    put(
        &mut menu,
        &player,
        BASE_SLOT,
        ItemStack::new(&vanilla_items::DIAMOND_HELMET),
    );

    let result = slot_item(&menu, RESULT_SLOT);
    assert!(result.is(&vanilla_items::NETHERITE_HELMET));
    assert_eq!(recipe_error(&menu), 0);
}

#[test]
fn quick_move_into_a_full_inventory_leaves_the_result_in_place() {
    let (_world, player, _pos, mut menu) = test_smithing("smithing_full_inventory");

    put(
        &mut menu,
        &player,
        TEMPLATE_SLOT,
        ItemStack::new(&vanilla_items::NETHERITE_UPGRADE_SMITHING_TEMPLATE),
    );
    put(
        &mut menu,
        &player,
        BASE_SLOT,
        ItemStack::new(&vanilla_items::DIAMOND_BOOTS),
    );
    put(
        &mut menu,
        &player,
        ADDITION_SLOT,
        ItemStack::new(&vanilla_items::NETHERITE_INGOT),
    );
    assert!(slot_item(&menu, RESULT_SLOT).is(&vanilla_items::NETHERITE_BOOTS));

    {
        let mut inventory = player.inventory.lock();
        for slot in 0..MAIN_INVENTORY_SLOT_COUNT {
            inventory.set_item(
                slot,
                ItemStack::with_count(&vanilla_items::STONE, FULL_STACK_COUNT),
            );
        }
    }

    menu.clicked(Click::QuickMove { slot: RESULT_SLOT }, &player)
        .expect("quick move click should be handled");

    assert!(
        slot_item(&menu, RESULT_SLOT).is(&vanilla_items::NETHERITE_BOOTS),
        "a full inventory must leave the result untouched"
    );
    assert!(
        player
            .inventory
            .lock()
            .items()
            .iter()
            .all(|item| !item.is(&vanilla_items::NETHERITE_BOOTS))
    );
}
