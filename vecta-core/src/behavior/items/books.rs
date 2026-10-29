//! Writable and written book item behaviors.

use std::sync::Arc;

use vecta_macros::item_behavior;
use vecta_protocol::packets::game::COpenBook;
use vecta_registry::data_components::vanilla_components::WRITTEN_BOOK_CONTENT;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::stat::vanilla_stat_types;

use crate::{
    behavior::{InteractionResult, ItemBehavior, UseItemContext},
    command::{
        execution::{CommandSource, CommandTextResolver},
        sender::CommandSender,
    },
    player::Player,
};

/// Behavior for the `minecraft:writable_book` item.
///
/// Vanilla `WritableBookItem.use` only awards the use statistic on the server;
/// the edit screen is opened by the client, which then reports the result with
/// `ServerboundEditBookPacket`.
#[item_behavior]
pub struct WritableBookItem;

impl ItemBehavior for WritableBookItem {
    fn use_item(&self, context: &mut UseItemContext) -> InteractionResult {
        let item = context.inv.with_item(|stack| stack.item());
        context
            .player
            .award_stat(&vanilla_stat_types::ITEM_USED, item);
        InteractionResult::Success
    }
}

/// Behavior for the `minecraft:written_book` item.
#[item_behavior]
pub struct WrittenBookItem;

impl ItemBehavior for WrittenBookItem {
    fn use_item(&self, context: &mut UseItemContext) -> InteractionResult {
        if context
            .inv
            .with_item(|stack| resolve_written_book(stack, context.player))
        {
            context.player.broadcast_inventory_changes();
        }

        if context
            .inv
            .with_item(|stack| stack.has(WRITTEN_BOOK_CONTENT))
        {
            context.player.send_packet(COpenBook { hand: context.hand });
        }

        let item = context.inv.with_item(|stack| stack.item());
        context
            .player
            .award_stat(&vanilla_stat_types::ITEM_USED, item);
        InteractionResult::Success
    }
}

/// Vanilla `ServerPlayer.openItemGui` / `WrittenBookContent.resolveForItem`.
///
/// Returns whether the stack's content changed, which the caller turns into a
/// container broadcast. A content that cannot be resolved is marked resolved
/// without rewriting its pages, exactly like vanilla.
fn resolve_written_book(stack: &mut ItemStack, player: &Arc<Player>) -> bool {
    let Some(content) = stack.get(WRITTEN_BOOK_CONTENT).cloned() else {
        return false;
    };
    if content.resolved() {
        return false;
    }

    // Resolution needs a command source, and a command source needs the
    // server. Detached players have none, so the content keeps its unresolved
    // state instead of a fabricated resolution result.
    let Some(server) = player.server.upgrade() else {
        return false;
    };

    let source = CommandSource::new(CommandSender::Player(Arc::clone(player)), server);
    let resolver = CommandTextResolver::with_entity_override(&source, player.as_ref());

    if let Some(resolved_content) = content.resolve(&resolver) {
        stack.set(WRITTEN_BOOK_CONTENT, resolved_content);
        true
    } else {
        stack.set(WRITTEN_BOOK_CONTENT, content.mark_resolved());
        false
    }
}

#[cfg(test)]
mod tests {
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
        let player = TestPlayerBuilder::new(Arc::clone(&fixture.world), format!("{name}{id}"), id)
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
}
