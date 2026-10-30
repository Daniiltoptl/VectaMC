use super::*;

use text_components::content::{Content, NbtSource, Resolvable};
use vecta_registry::data_components::components::{Filterable, WrittenBookContent};
use vecta_registry::data_components::vanilla_components::WRITTEN_BOOK_CONTENT;
use vecta_utils::types::InteractionHand;

use crate::behavior::InteractionResult;
use crate::bootstrap::init_globals;
use crate::player::game_mode::use_item;

fn nbt_page(source: NbtSource, path: &'static str) -> Filterable<TextComponent> {
    Filterable::pass_through(TextComponent {
        content: Content::Resolvable(Resolvable::NBT {
            path: path.into(),
            interpret: false,
            plain: true,
            separator: None,
            source,
        }),
        ..Default::default()
    })
}

fn unresolved_book(pages: Vec<Filterable<TextComponent>>) -> ItemStack {
    let mut book = ItemStack::new(&vanilla_items::WRITTEN_BOOK);
    book.set(
        WRITTEN_BOOK_CONTENT,
        WrittenBookContent::new(
            Filterable::pass_through("Test book".to_owned()),
            "Author".to_owned(),
            0,
            pages,
            false,
        )
        .expect("test written content should be valid"),
    );
    book
}

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
            let page = nbt_page(NbtSource::Entity("@s".into()), "SelectedItem.id");
            let stack = unresolved_book(vec![page]);
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

#[test]
fn lectern_book_resolves_self_and_relative_block_nbt_at_the_lectern() {
    use crate::behavior::{BLOCK_BEHAVIORS, BlockHitResult, InventoryAccess};
    use crate::block_entity::entities::LecternBlockEntity;
    use crate::test_support::insert_ready_full_chunk;
    use vecta_registry::blocks::properties::Direction;
    use vecta_utils::{ChunkPos, Downcast as _, types::UpdateFlags};

    init_globals();
    let primary_fixture = fresh_test_world("LecternPrimary");
    let fixture = fresh_test_derived_world(&primary_fixture.world, "LecternBookContext");
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime should initialize");
    runtime.block_on(async {
        let world = &fixture.world;
        let domain = ResolvedDomainConfig {
            name: world.domain().to_owned(),
            default_world: primary_fixture.world.key.clone(),
            worlds: vec![primary_fixture.world.key.clone(), world.key.clone()],
        };
        let server = test_server_with_worlds(
            domain.name.clone(),
            slice::from_ref(&domain),
            &[Arc::clone(&primary_fixture.world), Arc::clone(world)],
            PermissionSubjectIndex::new(),
        )
        .await
        .expect("test server should initialize");
        let player = test_player(&server, Arc::clone(world));
        assert!(server.online_players.insert(Arc::clone(&player)));
        assert!(world.add_player(Arc::clone(&player), ResetReason::InitialJoin));
        let _ = player.mark_joined_world();

        let pos = BlockPos::new(20, 65, 20);
        let below = pos.below();
        insert_ready_full_chunk(world, ChunkPos::from_block_pos(pos));
        let state = vanilla_blocks::LECTERN.default_state();
        assert!(world.set_block(pos, state, UpdateFlags::UPDATE_ALL));
        assert!(world.set_block(below, state, UpdateFlags::UPDATE_ALL));
        let block_entity = world
            .get_block_entity(below)
            .expect("lower lectern should exist");
        block_entity
            .downcast_ref::<LecternBlockEntity>()
            .expect("registered lectern has the correct implementation")
            .set_book(ItemStack::new(&vanilla_items::WRITABLE_BOOK));

        let pages = [
            (NbtSource::Entity("@s".into()), "SelectedItem.id"),
            (NbtSource::Block("~ ~-1 ~".into()), "Book.id"),
        ]
        .into_iter()
        .map(|(source, path)| nbt_page(source, path))
        .collect();
        let book = unresolved_book(pages);
        player
            .inventory
            .lock()
            .set_item_in_hand(InteractionHand::MainHand, book);
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);
        let hit = BlockHitResult {
            location: DVec3::new(20.5, 66.0, 20.5),
            direction: Direction::Up,
            block_pos: pos,
            miss: false,
            inside: false,
            world_border_hit: false,
        };
        assert_eq!(
            BLOCK_BEHAVIORS
                .get_behavior(&vanilla_blocks::LECTERN)
                .use_item_on(
                    state,
                    world,
                    pos,
                    &player,
                    InteractionHand::MainHand,
                    &hit,
                    &mut inventory,
                ),
            InteractionResult::Success
        );

        let placed = world.get_block_entity(pos).expect("lectern should exist");
        let book = placed
            .downcast_ref::<LecternBlockEntity>()
            .expect("lectern implementation")
            .book();
        let content = book
            .get(WRITTEN_BOOK_CONTENT)
            .expect("placed written content survives");
        assert!(content.resolved());
        assert_eq!(
            content.pages()[0].raw().to_string(),
            "\"minecraft:written_book\""
        );
        assert_eq!(
            content.pages()[1].raw().to_string(),
            "\"minecraft:writable_book\""
        );
    });
}
