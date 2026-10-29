use std::io::Cursor;
use std::sync::Arc;

use text_components::TextComponent;
use vecta_protocol::packet_traits::{CompressionInfo, EncodedPacket};
use vecta_registry::data_components::components::{Filterable, WrittenBookContent};
use vecta_registry::data_components::vanilla_components::WRITTEN_BOOK_CONTENT;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::packets::play::C_OPEN_BOOK;
use vecta_registry::stat::{Stat, vanilla_stat_types};
use vecta_registry::{init_vanilla_registry, vanilla_items};
use vecta_utils::codec::VarInt;
use vecta_utils::locks::SyncMutex;
use vecta_utils::serial::ReadFrom as _;
use vecta_utils::types::InteractionHand;

use crate::behavior::{InteractionResult, init_behaviors};
use crate::bootstrap::init_globals;
use crate::entity::next_entity_id;
use crate::player::connection::NetworkConnection;
use crate::player::game_mode::use_item;
use crate::player::{Player, PlayerConnection};
use crate::test_support::{TestPlayerBuilder, TestWorld, fresh_test_world};

#[derive(Default)]
struct RecordingConnection(SyncMutex<Vec<EncodedPacket>>);

impl NetworkConnection for Arc<RecordingConnection> {
    fn compression(&self) -> Option<CompressionInfo> {
        None
    }
    fn send_encoded(&self, packet: EncodedPacket) {
        self.0.lock().push(packet);
    }
    fn send_encoded_bundle(&self, packets: Vec<EncodedPacket>) {
        self.0.lock().extend(packets);
    }
    fn disconnect_with_reason(&self, _reason: TextComponent) {}
    fn tick(&self) {}
    fn latency(&self) -> i32 {
        0
    }
    fn close(&self) {}
    fn closed(&self) -> bool {
        false
    }
}

fn payloads(connection: &RecordingConnection, id: i32) -> Vec<Vec<u8>> {
    connection
        .0
        .lock()
        .iter()
        .filter_map(|packet| {
            let bytes = packet.encoded_data.as_slice();
            let mut cursor = Cursor::new(bytes);
            VarInt::read(&mut cursor).ok()?;
            if VarInt::read(&mut cursor).ok()?.0 != id {
                return None;
            }
            Some(bytes[cursor.position() as usize..].to_vec())
        })
        .collect()
}

fn test_player(name: &'static str) -> (Arc<Player>, Arc<RecordingConnection>, TestWorld) {
    init_globals();
    init_vanilla_registry();
    init_behaviors();
    let fixture = fresh_test_world(name);
    let connection = Arc::new(RecordingConnection::default());
    let id = next_entity_id();
    let player = TestPlayerBuilder::new(Arc::clone(&fixture.world), name, id)
        .connection(Arc::new(PlayerConnection::Other(Box::new(Arc::clone(
            &connection,
        )))))
        .build();
    (player, connection, fixture)
}

fn written_book(resolved: bool) -> ItemStack {
    let mut stack = ItemStack::new(&vanilla_items::WRITTEN_BOOK);
    stack.set(
        WRITTEN_BOOK_CONTENT,
        WrittenBookContent::new(
            Filterable::pass_through("Title".to_owned()),
            "Author".to_owned(),
            0,
            vec![Filterable::pass_through(TextComponent::plain("page"))],
            resolved,
        )
        .expect("test written book should be valid"),
    );
    stack
}

#[test]
fn using_a_written_book_opens_the_hand_that_holds_it() {
    for (hand, hand_id) in [
        (InteractionHand::MainHand, 0),
        (InteractionHand::OffHand, 1),
    ] {
        let (player, connection, fixture) = test_player("BookOpener");
        player
            .inventory
            .lock()
            .set_item_in_hand(hand, written_book(true));

        assert_eq!(
            use_item(&player, &fixture.world, hand),
            InteractionResult::Success
        );

        let packets = payloads(&connection, C_OPEN_BOOK);
        assert_eq!(packets.len(), 1, "one open-book packet per use");
        assert_eq!(
            VarInt::read(&mut Cursor::new(packets[0].as_slice()))
                .expect("hand id")
                .0,
            hand_id
        );
        assert!(player.stats().contains(&(
            Stat::new(&vanilla_stat_types::ITEM_USED, &vanilla_items::WRITTEN_BOOK),
            1
        )));
    }
}

#[test]
fn using_a_writable_book_only_awards_the_use_statistic() {
    let (player, connection, fixture) = test_player("BookWriter");
    let hand = InteractionHand::MainHand;
    player
        .inventory
        .lock()
        .set_item_in_hand(hand, ItemStack::new(&vanilla_items::WRITABLE_BOOK));

    assert_eq!(
        use_item(&player, &fixture.world, hand),
        InteractionResult::Success
    );

    assert_eq!(
        payloads(&connection, C_OPEN_BOOK).len(),
        0,
        "the client opens the writable-book editor itself"
    );
    assert!(player.stats().contains(&(
        Stat::new(
            &vanilla_stat_types::ITEM_USED,
            &vanilla_items::WRITABLE_BOOK
        ),
        1
    )));
}

#[test]
fn using_a_writable_book_with_written_content_opens_that_content() {
    let (player, connection, fixture) = test_player("BookDualContent");
    let mut stack = written_book(true);
    stack.set_item(&vanilla_items::WRITABLE_BOOK.key);
    player
        .inventory
        .lock()
        .set_item_in_hand(InteractionHand::OffHand, stack);

    assert_eq!(
        use_item(&player, &fixture.world, InteractionHand::OffHand),
        InteractionResult::Success
    );

    let packets = payloads(&connection, C_OPEN_BOOK);
    assert_eq!(packets.len(), 1);
    assert_eq!(
        VarInt::read(&mut Cursor::new(packets[0].as_slice()))
            .expect("hand should decode")
            .0,
        1
    );
    assert!(player.stats().contains(&(
        Stat::new(
            &vanilla_stat_types::ITEM_USED,
            &vanilla_items::WRITABLE_BOOK
        ),
        1
    )));
}

#[test]
fn a_written_book_without_content_does_not_send_the_open_book_packet() {
    let (player, connection, fixture) = test_player("BookEmpty");
    let hand = InteractionHand::MainHand;
    player
        .inventory
        .lock()
        .set_item_in_hand(hand, ItemStack::new(&vanilla_items::WRITTEN_BOOK));

    assert_eq!(
        use_item(&player, &fixture.world, hand),
        InteractionResult::Success
    );

    assert_eq!(payloads(&connection, C_OPEN_BOOK).len(), 0);
    assert!(player.stats().contains(&(
        Stat::new(&vanilla_stat_types::ITEM_USED, &vanilla_items::WRITTEN_BOOK),
        1
    )));
}

#[test]
fn unresolved_content_is_left_alone_without_a_command_source() {
    let (player, connection, fixture) = test_player("BookUnresolved");
    let hand = InteractionHand::MainHand;
    player
        .inventory
        .lock()
        .set_item_in_hand(hand, written_book(false));

    assert_eq!(
        use_item(&player, &fixture.world, hand),
        InteractionResult::Success
    );

    let stack = player.inventory.lock().get_item_in_hand(hand).clone();
    assert!(
        !stack
            .get(WRITTEN_BOOK_CONTENT)
            .expect("content should survive")
            .resolved(),
        "without a server handle the content must not be marked resolved"
    );
    assert_eq!(payloads(&connection, C_OPEN_BOOK).len(), 1);
}
