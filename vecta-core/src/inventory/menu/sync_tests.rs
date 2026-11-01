//! Menu-to-client synchronization (W04-019).
//!
//! Every test asserts what actually reached the client through a recording
//! connection, against vanilla `AbstractContainerMenu` and
//! `ServerGamePacketListenerImpl`.

use std::sync::Arc;
use std::sync::atomic::{AtomicI16, Ordering};

use vecta_protocol::packets::game::{ClickType, HashedPatchMap, HashedStack, SContainerClick};
use vecta_registry::RegistryEntry as _;
use vecta_registry::init_vanilla_registry;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::menu_type::MenuTypeRef;
use vecta_registry::vanilla_items;
use vecta_registry::vanilla_menu_types;
use vecta_utils::DowncastType;
use vecta_utils::DowncastTypeKey;
use vecta_utils::locks::{IntoShared as _, Shared};

use super::behavior::MenuBehavior;
use super::builder::{DataSlot, MenuBuilder};
use super::kind::MenuKind;
use super::kinds::BasicKind;
use crate::bootstrap::init_globals;
use crate::entity::next_entity_id;
use crate::inventory::container::{Container as _, SimpleContainer};
use crate::inventory::lock::ContainerLockGuard;
use crate::player::Player;
use crate::test_support::{
    ContainerContents, ContainerDataChange, SentPackets, TestPlayerBuilder, TestWorld,
    container_contents, container_data_changes, container_slot_changes, cursor_items,
    fresh_test_world, recording_connection,
};
use crate::world::World;

/// Slot count of the `generic_9x1` menu: nine container slots plus the
/// thirty-six player-inventory slots.
const GENERIC_9X1_SLOTS: usize = 45;

fn init() {
    init_globals();
    init_vanilla_registry();
}

fn test_player(world: &Arc<World>) -> (Arc<Player>, SentPackets) {
    let (connection, packets) = recording_connection();
    let player = TestPlayerBuilder::new(Arc::clone(world), "MenuSync", next_entity_id())
        .connection(connection)
        .build();
    (player, packets)
}

/// A pickup click on `slot` with an empty cursor and no client-side prediction.
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

/// What the client received when the menu opened.
struct Opening {
    contents: ContainerContents,
    slot_changes: usize,
    cursor_items: usize,
}

struct SyncFixture {
    player: Arc<Player>,
    packets: SentPackets,
    container: Shared<SimpleContainer>,
    opening: Opening,
    _world: TestWorld,
}

impl SyncFixture {
    /// The container id the server allocated when the menu was opened.
    fn container_id(&self) -> i32 {
        self.opening.contents.0
    }

    /// The state id the client was told about in the opening snapshot.
    fn state_id(&self) -> i32 {
        self.opening.contents.1
    }
}

/// Opens a `generic_9x1` menu whose first container slot holds three stone.
fn open_sync_menu(name: &'static str) -> SyncFixture {
    init();
    let world = fresh_test_world(name);
    let (player, packets) = test_player(&world.world);

    let container = SimpleContainer::new(9).into_shared();
    container
        .lock()
        .set_item(0, ItemStack::with_count(&vanilla_items::STONE, 3));

    let menu_container = Arc::clone(&container);
    player.open_menu("Sync", move |context| {
        let mut builder = MenuBuilder::new(&vanilla_menu_types::GENERIC_9X1, context.container_id);
        builder.section(menu_container, 9);
        builder.player_inventory(&context.player.inventory);
        builder.build(BasicKind)
    });

    let snapshots = container_contents(&packets);
    assert_eq!(
        snapshots.len(),
        1,
        "opening a menu sends exactly one full snapshot"
    );
    let opening = Opening {
        contents: snapshots.into_iter().next().expect("one snapshot"),
        slot_changes: container_slot_changes(&packets).len(),
        cursor_items: cursor_items(&packets).len(),
    };
    packets.lock().clear();

    SyncFixture {
        player,
        packets,
        container,
        opening,
        _world: world,
    }
}

#[test]
fn opening_a_menu_sends_one_full_snapshot_and_no_slot_updates() {
    let fixture = open_sync_menu("menu_sync_open");

    // The container counter starts at 1, so the first menu gets id 1, and
    // `setSynchronizer` -> `sendAllDataToRemote` bumps the state id to 1.
    assert_eq!(
        fixture.container_id(),
        1,
        "the first menu uses container id 1"
    );
    assert_eq!(
        fixture.state_id(),
        1,
        "the opening snapshot uses state id 1"
    );

    let (container_id, _, items, carried) = &fixture.opening.contents;
    assert_eq!(*container_id, fixture.container_id());
    assert_eq!(
        items.len(),
        GENERIC_9X1_SLOTS,
        "the snapshot covers every slot of the menu type"
    );
    assert_eq!(
        items[0],
        ItemStack::with_count(&vanilla_items::STONE, 3),
        "the container's contents arrive in menu order"
    );
    assert_eq!(
        items[9],
        ItemStack::empty(),
        "the player inventory follows the container slots"
    );
    assert!(carried.is_empty(), "nothing is on the cursor after opening");
    assert_eq!(
        fixture.opening.slot_changes, 0,
        "a full snapshot needs no per-slot packets"
    );
    assert_eq!(
        fixture.opening.cursor_items, 0,
        "the carried item travels inside the snapshot"
    );
}

#[test]
fn a_stale_state_id_resends_the_whole_menu() {
    let fixture = open_sync_menu("menu_sync_stale");

    // One behind the state the client was last told about.
    fixture.player.handle_container_click(pickup_click(
        fixture.container_id(),
        fixture.state_id() - 1,
        10,
    ));

    let snapshots = container_contents(&fixture.packets);
    assert_eq!(
        snapshots.len(),
        1,
        "a stale click forces `broadcastFullState`, i.e. one set-content"
    );
    let (container_id, state_id, items, carried) = &snapshots[0];
    assert_eq!(*container_id, fixture.container_id());
    assert_eq!(
        *state_id,
        fixture.state_id() + 1,
        "the resync increments the state id once"
    );
    assert_eq!(items.len(), GENERIC_9X1_SLOTS);
    assert!(carried.is_empty());
    assert!(
        container_slot_changes(&fixture.packets).is_empty(),
        "the full snapshot replaces per-slot updates"
    );
}

#[test]
fn a_current_state_id_only_sends_what_the_click_changed() {
    let fixture = open_sync_menu("menu_sync_current");

    // Picking up the stone moves it to the cursor, so exactly two things
    // changed: slot 0 and the carried item.
    fixture.player.handle_container_click(pickup_click(
        fixture.container_id(),
        fixture.state_id(),
        0,
    ));

    assert!(
        container_contents(&fixture.packets).is_empty(),
        "an up-to-date click never falls back to a full snapshot"
    );

    let slot_changes = container_slot_changes(&fixture.packets);
    assert_eq!(slot_changes.len(), 1, "only slot 0 changed");
    let (container_id, state_id, slot, item) = &slot_changes[0];
    assert_eq!(*container_id, fixture.container_id());
    assert_eq!(*slot, 0);
    assert!(item.is_empty(), "the picked-up stack leaves the slot empty");
    assert_eq!(
        *state_id,
        fixture.state_id() + 1,
        "each sent slot increments the state id, as `sendSlotChange` does"
    );

    let cursors = cursor_items(&fixture.packets);
    assert_eq!(cursors.len(), 1, "the cursor is synced once");
    assert_eq!(cursors[0], ItemStack::with_count(&vanilla_items::STONE, 3));
}

#[test]
fn a_client_slot_map_never_becomes_server_state() {
    let fixture = open_sync_menu("menu_sync_lying_client");

    // The client claims slot 0 holds sixty-four diamonds. Vanilla keeps that
    // hash only as "what the client believes" (`setRemoteSlotUnsafe`) and
    // never as container contents.
    let mut click = pickup_click(fixture.container_id(), fixture.state_id(), 0);
    click.changed_slots.insert(
        0,
        HashedStack::Item {
            item_id: vanilla_items::DIAMOND.id() as i32,
            count: 64,
            components: HashedPatchMap::default(),
        },
    );
    fixture.player.handle_container_click(click);

    // The real click (picking up three stone) is what the server applied.
    assert!(fixture.container.lock().get_item(0).is_empty());
    let cursors = cursor_items(&fixture.packets);
    assert_eq!(cursors.len(), 1);
    assert_eq!(
        cursors[0],
        ItemStack::with_count(&vanilla_items::STONE, 3),
        "the cursor holds what the server computed, not what the client claimed"
    );

    // And the client is corrected: its hash does not match the empty slot.
    let slot_changes = container_slot_changes(&fixture.packets);
    assert_eq!(slot_changes.len(), 1, "the wrong hash forces a slot resend");
    assert_eq!(slot_changes[0].2, 0);
    assert!(
        slot_changes[0].3.is_empty(),
        "the server sends its own contents, not the client's claim"
    );
}

#[test]
fn a_click_on_an_out_of_range_slot_is_rejected() {
    let fixture = open_sync_menu("menu_sync_bad_slot");

    // Vanilla `isValidSlotIndex` admits -1, -999 and every index below the
    // slot count, so the first invalid index is exactly the slot count.
    fixture.player.handle_container_click(pickup_click(
        fixture.container_id(),
        fixture.state_id(),
        GENERIC_9X1_SLOTS as i16,
    ));

    assert!(
        container_contents(&fixture.packets).is_empty(),
        "an invalid slot neither resyncs nor syncs"
    );
    assert!(
        container_slot_changes(&fixture.packets).is_empty(),
        "an invalid slot sends no slot update"
    );
    assert!(
        cursor_items(&fixture.packets).is_empty(),
        "an invalid slot sends no cursor update"
    );
    assert_eq!(
        fixture.container.lock().get_item(0),
        &ItemStack::with_count(&vanilla_items::STONE, 3),
        "the rejected click must not move anything"
    );
}

#[test]
fn a_click_for_another_window_is_ignored() {
    let fixture = open_sync_menu("menu_sync_foreign");

    fixture.player.handle_container_click(pickup_click(
        fixture.container_id() + 1,
        fixture.state_id(),
        0,
    ));

    assert!(
        container_contents(&fixture.packets).is_empty(),
        "a click for a foreign window sends nothing"
    );
    assert!(
        container_slot_changes(&fixture.packets).is_empty(),
        "a click for a foreign window sends no slot update"
    );
    assert!(
        cursor_items(&fixture.packets).is_empty(),
        "a click for a foreign window sends no cursor update"
    );
    assert_eq!(
        fixture.container.lock().get_item(0),
        &ItemStack::with_count(&vanilla_items::STONE, 3),
        "the foreign click must not touch the open menu's container"
    );
}

#[test]
fn a_server_side_slot_change_is_synced_once_on_the_next_tick() {
    let fixture = open_sync_menu("menu_sync_output");

    // A hopper, another player, or a block entity tick can change the
    // container behind the menu's back.
    fixture
        .container
        .lock()
        .set_item(3, ItemStack::with_count(&vanilla_items::DIRT, 5));

    // Vanilla `ServerPlayer.tick` -> `containerMenu.broadcastChanges()`.
    fixture.player.broadcast_inventory_changes();

    let slot_changes = container_slot_changes(&fixture.packets);
    assert_eq!(slot_changes.len(), 1, "one changed slot, one packet");
    let (container_id, state_id, slot, item) = &slot_changes[0];
    assert_eq!(*container_id, fixture.container_id());
    assert_eq!(*slot, 3);
    assert_eq!(*item, ItemStack::with_count(&vanilla_items::DIRT, 5));
    assert_eq!(*state_id, fixture.state_id() + 1);

    // The remote slot is now known, so a second tick with the same contents
    // sends nothing.
    fixture.player.broadcast_inventory_changes();
    assert_eq!(
        container_slot_changes(&fixture.packets).len(),
        1,
        "an unchanged slot is not resent"
    );
}

/// A menu kind that writes its one property slot on every tick, like a
/// furnace's cooking progress.
struct TickProperty {
    slot: DataSlot,
    value: Arc<AtomicI16>,
}

// SAFETY: This test-owned key uniquely identifies the concrete menu kind
// within the test process.
unsafe impl DowncastType for TickProperty {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:test/menu/sync/tick_property");
}

impl MenuKind for TickProperty {
    fn on_tick(
        &mut self,
        behavior: &mut MenuBehavior,
        _guard: &mut ContainerLockGuard,
        _player: &Player,
    ) {
        self.slot.set(behavior, self.value.load(Ordering::Acquire));
    }
}

struct DataFixture {
    player: Arc<Player>,
    packets: SentPackets,
    container_id: i32,
    value: Arc<AtomicI16>,
    _world: TestWorld,
}

fn open_data_menu(name: &'static str) -> DataFixture {
    init();
    let world = fresh_test_world(name);
    let (player, packets) = test_player(&world.world);
    let container = SimpleContainer::new(9).into_shared();
    let value = Arc::new(AtomicI16::new(5));
    let menu_value = Arc::clone(&value);

    player.open_menu("Properties", move |context| {
        let mut builder = MenuBuilder::new(&vanilla_menu_types::GENERIC_9X1, context.container_id);
        builder.section(container, 9);
        builder.player_inventory(&context.player.inventory);
        let slot = builder.data_slot(0);
        builder.build(TickProperty {
            slot,
            value: menu_value,
        })
    });

    let snapshots = container_contents(&packets);
    assert_eq!(snapshots.len(), 1);
    let container_id = snapshots[0].0;
    // `sendInitialData` also sends every data slot, so the client starts at 0.
    let initial_data: Vec<ContainerDataChange> = container_data_changes(&packets);
    assert_eq!(initial_data.len(), 1, "the snapshot sends every data slot");
    assert_eq!(initial_data[0], (container_id, 0, 0));
    packets.lock().clear();

    DataFixture {
        player,
        packets,
        container_id,
        value,
        _world: world,
    }
}

#[test]
fn a_property_change_is_synced_once_on_the_next_tick() {
    let fixture = open_data_menu("menu_sync_property");

    // The menu's own tick writes the property, then the player tick syncs it.
    fixture.player.tick_open_menu();
    fixture.player.broadcast_inventory_changes();

    let data_changes = container_data_changes(&fixture.packets);
    assert_eq!(data_changes.len(), 1, "one changed property, one packet");
    assert_eq!(data_changes[0].0, fixture.container_id);
    assert_eq!(data_changes[0].1, 0, "the property keeps its index");
    assert_eq!(data_changes[0].2, 5);

    // Same value on the next tick: nothing to send.
    fixture.player.tick_open_menu();
    fixture.player.broadcast_inventory_changes();
    assert_eq!(
        container_data_changes(&fixture.packets).len(),
        1,
        "an unchanged property is not resent"
    );

    fixture.value.store(7, Ordering::Release);
    fixture.player.tick_open_menu();
    fixture.player.broadcast_inventory_changes();
    let data_changes = container_data_changes(&fixture.packets);
    assert_eq!(data_changes.len(), 2);
    assert_eq!(data_changes[1].2, 7);
    assert!(
        container_slot_changes(&fixture.packets).is_empty(),
        "a property change alone must not resend any slot"
    );
}

#[test]
fn suppressed_updates_are_dropped_and_resent_after_resume() {
    init();
    let (connection, packets) = recording_connection();
    let container = SimpleContainer::new(9).into_shared();
    let mut builder = MenuBuilder::new(&vanilla_menu_types::GENERIC_9X1, 3);
    builder.section(Arc::clone(&container), 9);
    builder.section(SimpleContainer::new(36).into_shared(), 36);
    let mut menu = builder.build(BasicKind);

    menu.behavior_mut().send_all_data_to_remote(&connection);
    assert_eq!(container_contents(&packets).len(), 1);
    packets.lock().clear();

    // `handleContainerClick` suppresses updates around `clicked` so a single
    // click cannot emit half-applied state.
    menu.behavior_mut().suppress_remote_updates();
    container
        .lock()
        .set_item(4, ItemStack::with_count(&vanilla_items::DIRT, 2));
    menu.behavior_mut().broadcast_changes(&connection);
    assert!(
        container_slot_changes(&packets).is_empty(),
        "a suppressed menu sends no slot update"
    );
    assert!(
        container_contents(&packets).is_empty(),
        "a suppressed menu sends no snapshot"
    );

    menu.behavior_mut().resume_remote_updates();
    menu.behavior_mut().broadcast_changes(&connection);
    let slot_changes = container_slot_changes(&packets);
    assert_eq!(
        slot_changes.len(),
        1,
        "the change held back during suppression is sent after resume"
    );
    assert_eq!(slot_changes[0].2, 4);
    assert_eq!(
        slot_changes[0].3,
        ItemStack::with_count(&vanilla_items::DIRT, 2)
    );
}

#[test]
fn the_state_id_wraps_at_fifteen_bits() {
    init();
    let (connection, packets) = recording_connection();
    // No menu type, so the builder does not insist on a client layout; this
    // test only exercises `incrementStateId`.
    let mut builder = MenuBuilder::new(None::<MenuTypeRef>, 2);
    builder.section(SimpleContainer::new(1).into_shared(), 1);
    let mut menu = builder.build(BasicKind);

    // Vanilla `incrementStateId`: `stateId = stateId + 1 & 32767`.
    for _ in 0..=0x7FFF {
        menu.behavior_mut().send_all_data_to_remote(&connection);
    }

    assert_eq!(
        menu.behavior().state_id(),
        0,
        "32768 increments wrap a 15-bit state id back to zero"
    );
    let snapshots = container_contents(&packets);
    assert_eq!(snapshots.len(), 0x8000, "one snapshot per increment");
    assert_eq!(
        snapshots.last().expect("one snapshot per increment").1,
        0,
        "the packet carries the wrapped state id"
    );
}
