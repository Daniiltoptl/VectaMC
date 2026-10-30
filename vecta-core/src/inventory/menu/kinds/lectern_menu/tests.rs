use std::sync::Arc;

use crate::entity::Entity as _;
use glam::DVec3;
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::blocks::properties::BlockStateProperties;
use vecta_registry::data_components::components::{Filterable, WritableBookContent};
use vecta_registry::data_components::vanilla_components::WRITABLE_BOOK_CONTENT;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::{init_vanilla_registry, vanilla_blocks, vanilla_items};
use vecta_utils::types::UpdateFlags;
use vecta_utils::{BlockPos, ChunkPos};

use super::*;
use crate::block_entity::SharedBlockEntity;
use crate::bootstrap::init_globals;
use crate::inventory::click::{Click, MouseButton};
use crate::inventory::container::Container as _;
use crate::inventory::menu::Menu;
use crate::test_support::{
    TestPlayerBuilder, TestWorld, fresh_test_world, insert_ready_full_chunk,
};

const TEST_POS: BlockPos = BlockPos::new(4, 64, 4);
const BOOK_SLOT: usize = 0;
const PAGE_DATA_SLOT: usize = 0;

struct Fixture {
    world: TestWorld,
    player: Arc<Player>,
    entity: SharedBlockEntity,
}

fn fixture(key: &'static str) -> Fixture {
    init_vanilla_registry();
    init_globals();
    let world_fixture = fresh_test_world(key);
    let world = &world_fixture.world;
    insert_ready_full_chunk(world, ChunkPos::from_block_pos(TEST_POS));
    assert!(world.set_block(
        TEST_POS,
        vanilla_blocks::LECTERN.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    let entity = world
        .get_block_entity(TEST_POS)
        .expect("the lectern block creates a block entity");
    let player = TestPlayerBuilder::new(Arc::clone(world), "LecternTester", 1).build();
    player.base().set_position_local(DVec3::new(4.5, 64.0, 4.5));
    Fixture {
        world: world_fixture,
        player,
        entity,
    }
}

fn lectern_entity(fixture: &Fixture) -> &LecternBlockEntity {
    fixture
        .entity
        .downcast_ref::<LecternBlockEntity>()
        .expect("lectern block entity type")
}

fn menu_for(fixture: &Fixture) -> Menu {
    let container = ContainerRef::from_block_entity(Arc::clone(&fixture.entity))
        .expect("the lectern exposes its book container");
    lectern(1, container, Arc::clone(&fixture.entity))
}

fn writable_book(pages: usize) -> ItemStack {
    let mut book = ItemStack::new(&vanilla_items::WRITABLE_BOOK);
    book.set(
        WRITABLE_BOOK_CONTENT,
        WritableBookContent::new(
            (0..pages)
                .map(|page| Filterable::pass_through(format!("page {page}")))
                .collect(),
        )
        .expect("test pages are valid"),
    );
    book
}

fn page_of(menu: &Menu) -> i16 {
    menu.behavior()
        .get_data(PAGE_DATA_SLOT)
        .expect("the lectern always registers its page data slot")
}

fn has_book_state(fixture: &Fixture) -> bool {
    fixture
        .world
        .world
        .get_block_state(TEST_POS)
        .get_value(&BlockStateProperties::HAS_BOOK)
}

fn book_in_inventory(fixture: &Fixture) -> i32 {
    fixture
        .player
        .inventory
        .lock()
        .get_items()
        .iter()
        .filter(|item| item.item() == &*vanilla_items::WRITABLE_BOOK)
        .map(ItemStack::count)
        .sum()
}

#[test]
fn page_buttons_clamp_at_both_ends() {
    let fixture = fixture("lectern_pages");
    lectern_entity(&fixture).set_book(writable_book(3));
    let mut menu = menu_for(&fixture);

    assert_eq!(page_of(&menu), 0);
    assert!(menu.click_menu_button(&fixture.player, BUTTON_PREV_PAGE));
    assert_eq!(page_of(&menu), 0, "the first page cannot go lower");

    assert!(menu.click_menu_button(&fixture.player, BUTTON_NEXT_PAGE));
    assert_eq!(page_of(&menu), 1);
    assert!(menu.click_menu_button(&fixture.player, BUTTON_NEXT_PAGE));
    assert_eq!(page_of(&menu), 2);
    assert!(menu.click_menu_button(&fixture.player, BUTTON_NEXT_PAGE));
    assert_eq!(page_of(&menu), 2, "the last page cannot go higher");
    assert_eq!(lectern_entity(&fixture).page(), 2);
}

#[test]
fn jump_buttons_select_the_requested_page() {
    let fixture = fixture("lectern_jump");
    lectern_entity(&fixture).set_book(writable_book(4));
    let mut menu = menu_for(&fixture);

    assert!(menu.click_menu_button(&fixture.player, BUTTON_PAGE_JUMP_RANGE_START + 2));
    assert_eq!(page_of(&menu), 2);

    assert!(menu.click_menu_button(&fixture.player, BUTTON_PAGE_JUMP_RANGE_START + 50));
    assert_eq!(page_of(&menu), 3, "jump targets are clamped to the book");
}

#[test]
fn invalid_buttons_are_rejected() {
    let fixture = fixture("lectern_invalid_buttons");
    lectern_entity(&fixture).set_book(writable_book(3));
    let mut menu = menu_for(&fixture);

    for button in [0, 4, 99, -1] {
        assert!(
            !menu.click_menu_button(&fixture.player, button),
            "button {button} is not a lectern button"
        );
    }
    assert_eq!(page_of(&menu), 0);
    assert_eq!(lectern_entity(&fixture).page(), 0);
}

#[test]
fn taking_the_book_moves_it_to_the_inventory_and_clears_the_lectern() {
    let fixture = fixture("lectern_take_book");
    lectern_entity(&fixture).set_book(writable_book(2));
    let mut menu = menu_for(&fixture);
    assert!(has_book_state(&fixture));

    assert!(menu.click_menu_button(&fixture.player, BUTTON_TAKE_BOOK));

    assert_eq!(book_in_inventory(&fixture), 1);
    assert!(!lectern_entity(&fixture).has_book());
    assert!(
        !has_book_state(&fixture),
        "taking the book clears the lectern's has_book state"
    );
    assert!(
        !menu.still_valid(&fixture.player),
        "the menu closes once the book is gone"
    );
}

#[test]
fn taking_the_book_requires_build_permission() {
    let fixture = fixture("lectern_take_book_no_permission");
    lectern_entity(&fixture).set_book(writable_book(2));
    let mut menu = menu_for(&fixture);
    fixture.player.abilities.lock().may_build = false;

    assert!(!menu.click_menu_button(&fixture.player, BUTTON_TAKE_BOOK));
    assert_eq!(book_in_inventory(&fixture), 0);
    assert!(lectern_entity(&fixture).has_book());
}

#[test]
fn two_viewers_share_the_page() {
    let fixture = fixture("lectern_two_viewers");
    lectern_entity(&fixture).set_book(writable_book(5));
    let mut first = menu_for(&fixture);
    let mut second = menu_for(&fixture);

    assert_eq!(page_of(&second), 0);
    assert!(first.click_menu_button(&fixture.player, BUTTON_NEXT_PAGE));
    assert!(first.click_menu_button(&fixture.player, BUTTON_NEXT_PAGE));
    assert_eq!(page_of(&first), 2);
    assert_eq!(
        page_of(&second),
        0,
        "the other viewer only learns the page on its next tick"
    );

    second.on_tick(&fixture.player);
    assert_eq!(page_of(&second), 2);
}

#[test]
fn the_book_slot_never_accepts_placement() {
    let fixture = fixture("lectern_no_placement");
    lectern_entity(&fixture).set_book(writable_book(1));
    let mut menu = menu_for(&fixture);

    // Put a second book on the cursor and try to place it into the book slot.
    fixture
        .player
        .inventory
        .lock()
        .set_item(9, writable_book(1));
    menu.clicked(
        Click::Pickup {
            slot: BOOK_SLOT,
            button: MouseButton::Left,
        },
        &fixture.player,
    )
    .expect("pickup clicks are handled");
    menu.clicked(
        Click::Pickup {
            slot: BOOK_SLOT,
            button: MouseButton::Left,
        },
        &fixture.player,
    )
    .expect("pickup clicks are handled");

    assert_eq!(
        lectern_entity(&fixture).page_count(),
        1,
        "the lectern still holds its original book"
    );
}
