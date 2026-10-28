use std::sync::Arc;

use super::{MenuBuilder, kinds::BasicKind};
use crate::{
    behavior::init_behaviors,
    block_entity::init_block_entities,
    chunk::{Chunk, status::ChunkStatus},
    entity::{Entity as _, entities::ItemEntity},
    inventory::{
        click::{Click, DragKind, MouseButton, QuickCraft, SwapTarget},
        container::{Container as _, SimpleContainer},
        lock::{ContainerLockGuard, ContainerRef},
        slots::{NormalSlot, Slot, SlotStorage},
    },
    player::Player,
    test_support::{TestPlayerBuilder, TestWorld, fresh_test_world, insert_ready_full_chunk},
};
use glam::DVec3;
use vecta_registry::{
    data_components::components::BundleContents,
    data_components::vanilla_components::BUNDLE_CONTENTS, init_vanilla_registry,
    item_stack::ItemStack, item_stack_template::ItemStackTemplate, items::ItemRef, vanilla_blocks,
    vanilla_entities, vanilla_items, vanilla_menu_types,
};
use vecta_utils::locks::{IntoShared as _, Shared};
use vecta_utils::types::{GameType, UpdateFlags};
use vecta_utils::{ChunkPos, Downcast as _, DowncastType, DowncastTypeKey, WorldAabb};

struct SingleItemSlot {
    base: NormalSlot,
}

// SAFETY: This test-only key uniquely identifies `SingleItemSlot`.
unsafe impl DowncastType for SingleItemSlot {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:test/slot/single_item");
}

impl Slot for SingleItemSlot {
    fn storage(&self) -> &SlotStorage {
        self.base.storage()
    }

    fn get_item<'a>(&self, guard: &'a ContainerLockGuard) -> &'a ItemStack {
        self.base.get_item(guard)
    }

    fn get_item_mut<'a>(&self, guard: &'a mut ContainerLockGuard) -> &'a mut ItemStack {
        self.base.get_item_mut(guard)
    }

    fn set_item(&self, guard: &mut ContainerLockGuard, stack: ItemStack) {
        self.base.set_item(guard, stack);
    }

    fn get_max_stack_size(&self, _guard: &ContainerLockGuard) -> i32 {
        1
    }

    fn set_changed(&self, guard: &mut ContainerLockGuard) {
        self.base.set_changed(guard);
    }

    fn get_container_slot(&self) -> usize {
        self.base.get_container_slot()
    }
}

struct PartialSwapFixture {
    world_fixture: TestWorld,
    player: Arc<Player>,
    target: Shared<SimpleContainer>,
}

fn perform_partial_swap(world_name: &'static str, game_mode: GameType) -> PartialSwapFixture {
    init_vanilla_registry();
    let world_fixture = fresh_test_world(world_name);
    let world = &world_fixture.world;
    insert_ready_full_chunk(world, ChunkPos::new(0, 0));
    let player = TestPlayerBuilder::new(Arc::clone(world), "SwapTester", 1).build();
    player.restore_game_modes(game_mode, None);
    player.base().set_position_local(DVec3::new(0.5, 64.0, 0.5));
    {
        let mut inventory = player.inventory.lock();
        for slot in 0..inventory.get_container_size() {
            inventory.set_item(slot, ItemStack::with_count(&vanilla_items::DIRT, 64));
        }
        inventory.set_item(0, ItemStack::with_count(&vanilla_items::DIRT, 2));
    }

    let target = SimpleContainer::new(1).into_shared();
    target
        .lock()
        .set_item(0, ItemStack::new(&vanilla_items::STONE));
    let target_ref = ContainerRef::from(Arc::clone(&target));
    let mut builder = MenuBuilder::new(None, 1);
    let target_slots = builder.custom_section([SingleItemSlot {
        base: NormalSlot::new(target_ref, 0),
    }]);
    let mut menu = builder.build(BasicKind {});
    menu.clicked(
        Click::Swap {
            slot: target_slots.start(),
            with: SwapTarget::Hotbar(0),
        },
        &player,
    );

    PartialSwapFixture {
        world_fixture,
        player,
        target,
    }
}

#[test]
fn swap_locks_player_inventory_when_menu_has_no_inventory_slots() {
    init_vanilla_registry();
    let world_fixture = fresh_test_world("menu_swap_without_inventory_slots");
    let world = &world_fixture.world;
    let player = TestPlayerBuilder::new(Arc::clone(world), "SwapTester", 1).build();
    let container = SimpleContainer::new(45).into_shared();
    container
        .lock()
        .set_item(0, ItemStack::new(&vanilla_items::STONE));

    let mut builder = MenuBuilder::new(&vanilla_menu_types::GENERIC_9X1, 1);
    let menu_slots = builder.section(Arc::clone(&container), 45);
    let mut menu = builder.build(BasicKind {});

    menu.clicked(
        Click::Swap {
            slot: menu_slots.start(),
            with: SwapTarget::Hotbar(0),
        },
        &player,
    );

    assert!(container.lock().get_item(0).is_empty());
    assert!(
        player
            .inventory
            .lock()
            .get_item(0)
            .is(&vanilla_items::STONE)
    );
}

#[test]
fn draining_a_block_entity_slot_marks_its_chunk_dirty() {
    init_vanilla_registry();
    init_behaviors();
    init_block_entities();
    let world_fixture = fresh_test_world("persistent_menu_drain");
    let world = &world_fixture.world;
    let pos = vecta_utils::BlockPos::new(0, 64, 0);
    let holder = insert_ready_full_chunk(world, ChunkPos::from_block_pos(pos));
    assert!(world.set_block(
        pos,
        vanilla_blocks::BARREL.default_state(),
        UpdateFlags::UPDATE_NONE,
    ));
    let container = ContainerRef::from_block_entity(
        world
            .get_block_entity(pos)
            .expect("barrel should create its block entity"),
    )
    .expect("barrel should expose a container");
    {
        let mut guard = ContainerLockGuard::lock_all(&[&container]);
        assert!(guard.set_item(
            container.container_id(),
            0,
            ItemStack::new(&vanilla_items::STONE),
        ));
    }
    holder
        .try_chunk(ChunkStatus::Full)
        .expect("full chunk should remain loaded")
        .clear_dirty();

    let player = TestPlayerBuilder::new(Arc::clone(world), "DrainTester", 1).build();
    player.base().set_position_local(DVec3::new(0.5, 64.0, 0.5));
    let mut builder = MenuBuilder::new(None, 1);
    let drained = builder.section(container, 1);
    builder.drain(drained);
    let mut menu = builder.build(BasicKind {});

    menu.removed(&player);

    assert!(
        holder
            .try_chunk(ChunkStatus::Full)
            .is_some_and(Chunk::is_dirty)
    );
}

#[test]
fn one_slot_creative_clone_drag_is_a_vanilla_noop() {
    init_vanilla_registry();
    let world_fixture = fresh_test_world("one_slot_clone_drag");
    let world = &world_fixture.world;
    let player = TestPlayerBuilder::new(Arc::clone(world), "CloneTester", 1).build();
    player.restore_game_modes(GameType::Creative, None);
    let container = SimpleContainer::new(1).into_shared();
    container
        .lock()
        .set_item(0, ItemStack::new(&vanilla_items::STONE));
    let mut builder = MenuBuilder::new(None, 1);
    let slots = builder.section(Arc::clone(&container), 1);
    let mut menu = builder.build(BasicKind {});
    *menu.behavior_mut().carried_mut() = ItemStack::new(&vanilla_items::STONE);

    menu.clicked(
        Click::QuickCraft(QuickCraft::Start {
            kind: DragKind::Clone,
        }),
        &player,
    );
    menu.clicked(
        Click::QuickCraft(QuickCraft::AddSlot {
            slot: slots.start(),
        }),
        &player,
    );
    menu.clicked(Click::QuickCraft(QuickCraft::End), &player);

    assert_eq!(container.lock().get_item(0).count(), 1);
    assert_eq!(menu.behavior().carried().count(), 1);
    assert_eq!(menu.behavior().quickcraft(), None);
}

#[test]
fn partial_swap_overflow_marks_displaced_item_as_thrown() {
    let fixture = perform_partial_swap("menu_partial_swap_overflow", GameType::Survival);
    let player_id = fixture.player.gameprofile.id();

    assert_eq!(fixture.player.inventory.lock().get_item(0).count(), 1);
    let target_item = fixture.target.lock().get_item(0).clone();
    assert!(target_item.is(&vanilla_items::DIRT));
    assert_eq!(target_item.count(), 1);
    let dropped = fixture.world_fixture.world.get_entities_in_aabb_matching(
        &WorldAabb::new(-2.0, 62.0, -2.0, 2.0, 68.0, 2.0),
        |entity| entity.entity_type() == &vanilla_entities::ITEM,
    );
    assert_eq!(dropped.len(), 1);
    let Some(item) = dropped[0].as_ref().downcast_ref::<ItemEntity>() else {
        panic!("dropped entity should retain its concrete item type");
    };
    assert!(item.get_item().is(&vanilla_items::STONE));
    assert_eq!(item.get_thrower(), Some(player_id));
}

#[test]
fn partial_swap_overflow_is_discarded_in_creative() {
    let fixture = perform_partial_swap("creative_menu_partial_swap_overflow", GameType::Creative);

    assert_eq!(fixture.player.inventory.lock().get_item(0).count(), 1);
    let target_item = fixture.target.lock().get_item(0).clone();
    assert!(target_item.is(&vanilla_items::DIRT));
    assert_eq!(target_item.count(), 1);
    assert!(
        fixture
            .player
            .inventory
            .lock()
            .items()
            .iter()
            .all(|item| !item.is(&vanilla_items::STONE))
    );
    assert!(
        fixture
            .world_fixture
            .world
            .get_entities_in_aabb_matching(
                &WorldAabb::new(-2.0, 62.0, -2.0, 2.0, 68.0, 2.0),
                |entity| entity.entity_type() == &vanilla_entities::ITEM,
            )
            .is_empty()
    );
}

/// Builds a one-slot menu backed by `container` and a matching player.
struct BundleFixture {
    /// Keeps the fixture world alive for the duration of the test.
    _world_fixture: TestWorld,
    player: Arc<Player>,
    container: Shared<SimpleContainer>,
    menu: super::Menu,
    slot: usize,
}

fn bundle_fixture(world_name: &'static str) -> BundleFixture {
    init_vanilla_registry();
    init_behaviors();
    let world_fixture = fresh_test_world(world_name);
    let world = &world_fixture.world;
    insert_ready_full_chunk(world, ChunkPos::new(0, 0));
    let player = TestPlayerBuilder::new(Arc::clone(world), "BundleTester", 1).build();
    player.base().set_position_local(DVec3::new(0.5, 64.0, 0.5));

    let container = SimpleContainer::new(2).into_shared();
    let container_ref = ContainerRef::from(Arc::clone(&container));
    let mut builder = MenuBuilder::new(None, 1);
    let slots = builder.custom_section([
        NormalSlot::new(container_ref.clone(), 0),
        NormalSlot::new(container_ref, 1),
    ]);
    let menu = builder.build(BasicKind {});

    BundleFixture {
        _world_fixture: world_fixture,
        player,
        container,
        menu,
        slot: slots.start(),
    }
}

/// Builds a bundle stack holding `count` of `item` inside its contents.
fn bundle_with(item: ItemRef, count: i32) -> ItemStack {
    let mut bundle = ItemStack::new(&vanilla_items::BUNDLE);
    bundle.set(
        BUNDLE_CONTENTS,
        BundleContents::new(vec![ItemStackTemplate::with_count(item, count)]),
    );
    bundle
}

fn bundle_contents(stack: &ItemStack) -> Vec<ItemStack> {
    stack
        .get(BUNDLE_CONTENTS)
        .expect("stack should keep bundle contents")
        .items()
        .iter()
        .map(ItemStackTemplate::create)
        .collect()
}

#[test]
fn bundle_on_cursor_moves_slot_items_into_its_contents() {
    let mut fixture = bundle_fixture("menu_bundle_cursor_insert");
    fixture
        .container
        .lock()
        .set_item(0, ItemStack::with_count(&vanilla_items::STONE, 5));
    *fixture.menu.behavior_mut().carried_mut() = ItemStack::new(&vanilla_items::BUNDLE);

    fixture.menu.clicked(
        Click::Pickup {
            slot: fixture.slot,
            button: MouseButton::Left,
        },
        &fixture.player,
    );

    assert!(
        fixture.container.lock().get_item(0).is_empty(),
        "the clicked slot must give up its stack"
    );
    let contents = bundle_contents(fixture.menu.behavior().carried());
    assert_eq!(contents.len(), 1);
    assert!(contents[0].is(&vanilla_items::STONE));
    assert_eq!(contents[0].count(), 5);
}

#[test]
fn bundle_in_slot_hands_one_stack_to_the_cursor() {
    let mut fixture = bundle_fixture("menu_bundle_slot_remove");
    fixture
        .container
        .lock()
        .set_item(0, bundle_with(&vanilla_items::STONE, 7));

    fixture.menu.clicked(
        Click::Pickup {
            slot: fixture.slot,
            button: MouseButton::Right,
        },
        &fixture.player,
    );

    let carried = fixture.menu.behavior().carried().clone();
    assert!(carried.is(&vanilla_items::STONE));
    assert_eq!(carried.count(), 7);
    assert!(
        bundle_contents(&fixture.container.lock().get_item(0).clone()).is_empty(),
        "the removed stack must leave the contents"
    );
}

#[test]
fn bundle_refuses_items_that_cannot_enter_containers() {
    let mut fixture = bundle_fixture("menu_bundle_refused_item");
    fixture
        .container
        .lock()
        .set_item(0, ItemStack::new(&vanilla_items::SHULKER_BOX));
    *fixture.menu.behavior_mut().carried_mut() = ItemStack::new(&vanilla_items::BUNDLE);

    fixture.menu.clicked(
        Click::Pickup {
            slot: fixture.slot,
            button: MouseButton::Left,
        },
        &fixture.player,
    );

    assert!(
        bundle_contents(fixture.menu.behavior().carried()).is_empty(),
        "a shulker box must not enter a bundle"
    );
    assert!(
        fixture
            .container
            .lock()
            .get_item(0)
            .is(&vanilla_items::SHULKER_BOX),
        "the refused item must stay in its slot"
    );
}

#[test]
fn bundle_keeps_the_remainder_when_it_fills_up() {
    let mut fixture = bundle_fixture("menu_bundle_overflow");
    fixture
        .container
        .lock()
        .set_item(0, ItemStack::with_count(&vanilla_items::STONE, 64));
    *fixture.menu.behavior_mut().carried_mut() = bundle_with(&vanilla_items::STONE, 40);

    fixture.menu.clicked(
        Click::Pickup {
            slot: fixture.slot,
            button: MouseButton::Left,
        },
        &fixture.player,
    );

    let slot_item = fixture.container.lock().get_item(0).clone();
    assert_eq!(
        slot_item.count(),
        40,
        "only the amount that still fits may be taken"
    );
    let contents = bundle_contents(fixture.menu.behavior().carried());
    assert_eq!(
        contents.len(),
        1,
        "the inserted stack merges into one stack"
    );
    assert_eq!(
        contents[0].count(),
        64,
        "the bundle holds exactly one full stack"
    );
}

#[test]
fn selecting_bundle_contents_only_touches_bundle_slots() {
    let mut fixture = bundle_fixture("menu_bundle_selection");
    fixture
        .container
        .lock()
        .set_item(0, bundle_with(&vanilla_items::STONE, 3));
    fixture
        .container
        .lock()
        .set_item(1, ItemStack::new(&vanilla_items::DIRT));

    fixture.menu.set_selected_bundle_item_index(0, 0);
    fixture.menu.set_selected_bundle_item_index(1, 0);

    let bundle = fixture.container.lock().get_item(0).clone();
    let selected = bundle
        .get(BUNDLE_CONTENTS)
        .expect("bundle contents should survive")
        .selected_item_index();
    assert_eq!(selected, 0);
    assert!(
        fixture
            .container
            .lock()
            .get_item(1)
            .is(&vanilla_items::DIRT),
        "a non-bundle slot must be left untouched"
    );
}
