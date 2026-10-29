use super::*;

use text_components::content::{Content, NbtSource, Resolvable};
use vecta_registry::data_components::components::{Filterable, WrittenBookContent};
use vecta_registry::data_components::vanilla_components::WRITTEN_BOOK_CONTENT;
use vecta_utils::types::InteractionHand;

use crate::behavior::InteractionResult;
use crate::bootstrap::init_globals;
use crate::player::game_mode::use_item;

#[test]
fn unresolved_book_can_read_its_holders_inventory() {
    init_globals();
    let fixture = fresh_test_world("BookInventoryNbt");
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime should initialize");
    runtime.block_on(async {
        let uuid = Uuid::new_v4();
        let mut permissions = PermissionSubjectIndex::new();
        permissions.set(
            uuid,
            PermissionSubjectState::new(vec![OP_GROUP.to_owned()], PermissionSet::new()),
        );
        let server = test_server(Arc::clone(&fixture.world), permissions)
            .await
            .expect("test server should initialize");
        let (player, _) = test_player_with_uuid_and_packets(
            &server,
            Arc::clone(&fixture.world),
            uuid,
            "BookNbt",
            next_entity_id(),
        );
        assert!(server.online_players.insert(Arc::clone(&player)));
        assert!(
            fixture
                .world
                .add_player(Arc::clone(&player), ResetReason::InitialJoin)
        );
        let _ = player.mark_joined_world();

        for hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
            let mut stack = ItemStack::new(&vanilla_items::WRITTEN_BOOK);
            let page = TextComponent {
                content: Content::Resolvable(Resolvable::NBT {
                    path: "SelectedItem.id".into(),
                    interpret: false,
                    plain: true,
                    separator: None,
                    source: NbtSource::Entity("@s".into()),
                }),
                ..Default::default()
            };
            stack.set(
                WRITTEN_BOOK_CONTENT,
                WrittenBookContent::new(
                    Filterable::pass_through("Inventory".to_owned()),
                    "Author".to_owned(),
                    0,
                    vec![Filterable::pass_through(page)],
                    false,
                )
                .expect("NBT book content should be valid"),
            );
            {
                let mut inventory = player.inventory.lock();
                inventory.set_item_in_hand(
                    InteractionHand::MainHand,
                    ItemStack::new(&vanilla_items::DIAMOND),
                );
                inventory.set_item_in_hand(hand, stack);
            }

            assert_eq!(
                use_item(&player, &fixture.world, hand),
                InteractionResult::Success
            );

            let inventory = player.inventory.lock();
            let content = inventory
                .get_item_in_hand(hand)
                .get(WRITTEN_BOOK_CONTENT)
                .expect("book content should survive");
            assert!(content.resolved());
            assert_eq!(
                content.pages()[0].raw().to_string(),
                if hand == InteractionHand::MainHand {
                    "\"minecraft:written_book\""
                } else {
                    "\"minecraft:diamond\""
                }
            );
        }
    });
}
