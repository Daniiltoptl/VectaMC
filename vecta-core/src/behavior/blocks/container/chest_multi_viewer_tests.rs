//! Two players at one live container (W04-020).
//!
//! A chest has one shared `Container` but one menu per viewer: the contents are
//! common, the cursor and the client's view are not. These tests drive two
//! harness players against the same block entity, which is the closest stand-in
//! for two real clients.

use std::sync::Arc;

use glam::DVec3;
use vecta_protocol::packets::game::{ClickType, HashedStack, SContainerClick};
use vecta_registry::blocks::properties::Direction;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::{vanilla_blocks, vanilla_items};
use vecta_utils::types::{InteractionHand, UpdateFlags};
use vecta_utils::{BlockPos, ChunkPos, Downcast as _};

use crate::behavior::InventoryAccess;
use crate::behavior::block::BlockBehavior;
use crate::behavior::blocks::container::ChestBlock;
use crate::behavior::context::{BlockHitResult, InteractionResult};
use crate::block_entity::entities::ChestBlockEntity;
use crate::bootstrap::init_globals;
use crate::entity::Entity as _;
use crate::entity::entities::ItemEntity;
use crate::entity::next_entity_id;
use crate::inventory::lock::ContainerLockGuard;
use crate::player::{Player, ResetReason};
use crate::test_support::{
    SentPackets, TestPlayerBuilder, TestWorld, container_contents, container_slot_changes,
    cursor_items, dropped_items, fresh_test_world, insert_ready_full_chunk, recording_connection,
};
use crate::world::World;

const TEST_POS: BlockPos = BlockPos::new(4, 64, 4);

/// The first player-inventory slot of a chest menu: twenty-seven chest slots
/// come before it.
const FIRST_INVENTORY_SLOT: i16 = 27;

fn stone(count: i32) -> ItemStack {
    ItemStack::with_count(&vanilla_items::STONE, count)
}

fn dirt(count: i32) -> ItemStack {
    ItemStack::with_count(&vanilla_items::DIRT, count)
}

fn chest_hit() -> BlockHitResult {
    BlockHitResult {
        location: DVec3::new(4.5, 64.5, 5.0),
        direction: Direction::South,
        block_pos: TEST_POS,
        miss: false,
        inside: false,
        world_border_hit: false,
    }
}

fn use_chest(world: &Arc<World>, player: &Arc<Player>) -> InteractionResult {
    let mut inventory =
        InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);
    ChestBlock::new(&vanilla_blocks::CHEST).use_without_item(
        world.get_block_state(TEST_POS),
        world,
        TEST_POS,
        player,
        &chest_hit(),
        &mut inventory,
    )
}

/// How many items of `template`'s kind this player holds, cursor excluded.
fn held(player: &Arc<Player>, template: &ItemStack) -> i32 {
    let inventory = player.inventory.lock();
    inventory
        .get_items()
        .iter()
        .filter(|stack| !stack.is_empty() && ItemStack::is_same_item(stack, template))
        .map(|stack| stack.count)
        .sum()
}

/// The state id the client would echo back: the newest one it was sent.
fn latest_state_id(packets: &SentPackets) -> i32 {
    let mut latest = 0;
    for (_, state_id, _, _) in container_contents(packets) {
        latest = latest.max(state_id);
    }
    for (_, state_id, _, _) in container_slot_changes(packets) {
        latest = latest.max(state_id);
    }
    latest
}

fn container_id(packets: &SentPackets) -> i32 {
    container_contents(packets).last().map_or_else(
        || panic!("the menu was opened, so a snapshot was sent"),
        |it| it.0,
    )
}

fn pickup_click(container_id: i32, state_id: i32, slot: i16) -> SContainerClick {
    SContainerClick {
        container_id,
        state_id,
        slot_num: slot,
        button_num: 0,
        click_type: ClickType::Pickup,
        changed_slots: rustc_hash::FxHashMap::default(),
        carried_item: HashedStack::Empty,
    }
}

/// Clicks `slot` for `player` using the state id its client currently believes.
fn click(player: &Arc<Player>, packets: &SentPackets, slot: i16) {
    let container_id = container_id(packets);
    let state_id = latest_state_id(packets);
    player.handle_container_click(pickup_click(container_id, state_id, slot));
}

struct TwoViewers {
    world: TestWorld,
    first: Arc<Player>,
    second: Arc<Player>,
    first_packets: SentPackets,
    second_packets: SentPackets,
}

fn two_viewer_chest(name: &'static str, slots: &[(usize, ItemStack)]) -> TwoViewers {
    init_globals();
    let world = fresh_test_world(name);
    insert_ready_full_chunk(&world.world, ChunkPos::from_block_pos(TEST_POS));
    assert!(world.world.set_block(
        TEST_POS,
        vanilla_blocks::CHEST.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));

    {
        let block_entity = world
            .world
            .get_block_entity(TEST_POS)
            .expect("a placed chest has a block entity");
        let container = block_entity
            .container_ref()
            .expect("a chest exposes its container");
        let mut guard = ContainerLockGuard::lock_all(&[&container]);
        let slots_view = guard
            .get_mut(container.container_id())
            .expect("the chest container is locked");
        for (slot, item) in slots {
            slots_view.set_item(*slot, item.clone());
        }
    }

    let (first, first_packets) = viewer(&world.world, "ChestFirst", 4.5);
    let (second, second_packets) = viewer(&world.world, "ChestSecond", 5.4);

    assert_eq!(
        use_chest(&world.world, &first),
        InteractionResult::Success,
        "the first viewer opens the chest"
    );
    assert_eq!(
        use_chest(&world.world, &second),
        InteractionResult::Success,
        "the second viewer opens the same chest"
    );

    // The opening snapshot is kept: it carries the container id and the first
    // state id, which is what each client echoes back on its next click.
    TwoViewers {
        world,
        first,
        second,
        first_packets,
        second_packets,
    }
}

fn viewer(world: &Arc<World>, name: &'static str, x: f64) -> (Arc<Player>, SentPackets) {
    let (connection, packets) = recording_connection();
    let player = TestPlayerBuilder::new(Arc::clone(world), name, next_entity_id())
        .connection(connection)
        .build();
    player
        .try_set_position(DVec3::new(x, 65.0, 4.5))
        .expect("the chest's chunk is loaded, so the viewer can stand there");
    assert!(
        world.add_player(Arc::clone(&player), ResetReason::InitialJoin),
        "the viewer has to be tracked so the opener recount can find it"
    );
    (player, packets)
}

fn open_count(world: &TestWorld) -> i32 {
    world
        .world
        .get_block_entity(TEST_POS)
        .expect("a placed chest has a block entity")
        .downcast_ref::<ChestBlockEntity>()
        .expect("the chest block must create a chest block entity")
        .open_count()
}

/// Reads one slot straight out of the shared container, behind both menus.
fn chest_slot(world: &TestWorld, slot: usize) -> ItemStack {
    let block_entity = world
        .world
        .get_block_entity(TEST_POS)
        .expect("the chest is still placed");
    let container = block_entity
        .container_ref()
        .expect("a chest exposes its container");
    let guard = ContainerLockGuard::lock_all(&[&container]);
    guard
        .get(container.container_id())
        .expect("the chest container is locked")
        .get_item(slot)
        .clone()
}

/// Writes one slot straight into the shared container, the way a hopper or a
/// command would while both viewers have the chest open.
fn set_chest_slot(world: &TestWorld, slot: usize, item: ItemStack) {
    let block_entity = world
        .world
        .get_block_entity(TEST_POS)
        .expect("the chest is still placed");
    let container = block_entity
        .container_ref()
        .expect("a chest exposes its container");
    let mut guard = ContainerLockGuard::lock_all(&[&container]);
    guard
        .get_mut(container.container_id())
        .expect("the chest container is locked")
        .set_item(slot, item);
}

#[test]
fn both_viewers_count_as_watchers_until_the_last_one_leaves() {
    let fixture = two_viewer_chest("chest_two_viewers_watchers", &[]);

    assert_eq!(
        open_count(&fixture.world),
        2,
        "vanilla counts both openers, not one"
    );

    fixture.first.close_container();
    assert_eq!(
        open_count(&fixture.world),
        1,
        "the chest stays open for the remaining viewer"
    );

    fixture.second.close_container();
    assert_eq!(
        open_count(&fixture.world),
        0,
        "the last viewer releases the chest"
    );
}

#[test]
fn interleaved_clicks_from_two_viewers_share_one_container() {
    let fixture = two_viewer_chest(
        "chest_two_viewers_interleaved",
        &[(0, stone(3)), (1, dirt(5))],
    );

    // The first viewer takes the stone and drops it into its own inventory.
    click(&fixture.first, &fixture.first_packets, 0);
    click(&fixture.first, &fixture.first_packets, FIRST_INVENTORY_SLOT);
    assert_eq!(held(&fixture.first, &stone(3)), 3);

    // The other viewer learns about it on its own tick, not from the click.
    fixture.second.broadcast_inventory_changes();
    assert!(
        container_slot_changes(&fixture.second_packets)
            .iter()
            .any(|(_, _, slot, item)| *slot == 0 && item.is_empty()),
        "a change made by one viewer reaches the other viewer's client"
    );

    // An outside change (a hopper, a command) reaches both viewers.
    set_chest_slot(
        &fixture.world,
        2,
        ItemStack::with_count(&vanilla_items::GOLD_INGOT, 2),
    );
    fixture.first.broadcast_inventory_changes();
    fixture.second.broadcast_inventory_changes();
    for packets in [&fixture.first_packets, &fixture.second_packets] {
        assert!(
            container_slot_changes(packets)
                .iter()
                .any(|(_, _, slot, item)| {
                    *slot == 2
                        && item.count == 2
                        && ItemStack::is_same_item(
                            item,
                            &ItemStack::new(&vanilla_items::GOLD_INGOT),
                        )
                }),
            "an external change reaches every open viewer"
        );
    }

    // The second viewer takes the dirt, without having clicked anything yet.
    click(&fixture.second, &fixture.second_packets, 1);
    click(
        &fixture.second,
        &fixture.second_packets,
        FIRST_INVENTORY_SLOT,
    );
    assert_eq!(held(&fixture.second, &dirt(5)), 5);
    assert_eq!(
        held(&fixture.second, &stone(3)),
        0,
        "the second viewer's cursor is its own; it never held the stone"
    );

    // Both taken slots are empty in the shared container, the outside item is
    // untouched.
    assert!(
        chest_slot(&fixture.world, 0).is_empty(),
        "the stone left the chest"
    );
    assert!(
        chest_slot(&fixture.world, 1).is_empty(),
        "the dirt left the chest"
    );
    assert_eq!(chest_slot(&fixture.world, 2).count, 2);
}

#[test]
fn a_stale_preview_cannot_duplicate_items_when_the_chest_breaks() {
    let fixture = two_viewer_chest("chest_two_viewers_stale", &[(0, stone(3))]);

    // The first viewer empties the chest.
    click(&fixture.first, &fixture.first_packets, 0);
    click(&fixture.first, &fixture.first_packets, FIRST_INVENTORY_SLOT);
    assert_eq!(held(&fixture.first, &stone(3)), 3);

    // The second viewer still shows the stack on its screen and clicks it.
    // Its own state id is current, so this is a stale *preview*, not a stale
    // window: the server must not hand out the items twice.
    click(&fixture.second, &fixture.second_packets, 0);

    assert!(
        cursor_items(&fixture.second_packets).is_empty(),
        "the second viewer gets no stack on its cursor"
    );
    let corrections = container_slot_changes(&fixture.second_packets);
    assert!(
        corrections
            .iter()
            .any(|(_, _, slot, item)| *slot == 0 && item.is_empty()),
        "the server corrects the second viewer's stale slot 0"
    );
    assert_eq!(held(&fixture.second, &stone(3)), 0);

    // Breaking the block must not pay out the stack a second time.
    assert!(fixture.world.world.set_block(
        TEST_POS,
        vanilla_blocks::AIR.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    fixture.first.tick_open_menu();
    fixture.second.tick_open_menu();

    assert!(
        !fixture.first.has_container_open() && !fixture.second.has_container_open(),
        "vanilla closes a menu whose container went away"
    );

    let dropped: i32 = dropped_items(&fixture.world.world, DVec3::new(4.5, 64.5, 4.5))
        .iter()
        .filter_map(|entity| entity.downcast_ref::<ItemEntity>())
        .map(ItemEntity::get_item)
        .filter(|stack| ItemStack::is_same_item(stack, &stone(3)))
        .map(|stack| stack.count)
        .sum();
    assert_eq!(
        held(&fixture.first, &stone(3)) + held(&fixture.second, &stone(3)) + dropped,
        3,
        "the three stone exist exactly once across both viewers and the drop"
    );
}
