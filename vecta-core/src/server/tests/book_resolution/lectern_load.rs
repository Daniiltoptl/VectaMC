use super::*;

use simdnbt::ToNbtTag as _;
use simdnbt::borrow::read_compound;
use vecta_utils::{Downcast as _, Identifier, nbt::to_canonical_snbt};

use crate::block_entity::{
    BLOCK_ENTITIES, BlockEntityLoadContext, SharedBlockEntity, entities::LecternBlockEntity,
};
use crate::scoreboard::ScoreHolder;
use crate::server::test_server_with_worlds_and_config;
use crate::test_support::advance_test_game_time_to;

const POS: BlockPos = BlockPos::new(20, 65, 20);

fn with_world(test: impl FnOnce(&Arc<Server>, &Arc<World>, &Arc<Player>)) {
    init_globals();
    let primary = fresh_test_world("LecternLoadPrimary");
    let fixture = fresh_test_derived_world(&primary.world, "LecternLoadTarget");
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime");
    runtime.block_on(async {
        let world = &fixture.world;
        let domain = ResolvedDomainConfig {
            name: world.domain().to_owned(),
            default_world: primary.world.key.clone(),
            worlds: vec![primary.world.key.clone(), world.key.clone()],
        };
        let server = test_server_with_worlds_and_config(
            domain.name.clone(),
            slice::from_ref(&domain),
            &[Arc::clone(&primary.world), Arc::clone(world)],
            PermissionSubjectIndex::new(),
            test_runtime_config_with_max_players(2),
        )
        .await
        .expect("test server");
        for x in 0..=2 {
            for z in 0..=2 {
                insert_ready_full_chunk(world, ChunkPos::new(x, z));
            }
        }
        let player = test_player(&server, Arc::clone(world));
        player.base().set_position_local(POS.get_center().into());
        assert!(server.online_players.insert(Arc::clone(&player)));
        assert!(world.add_player(Arc::clone(&player), ResetReason::InitialJoin));
        let _ = player.mark_joined_world();
        test(&server, world, &player);
    });
}

fn data(book: ItemStack, page: i32) -> NbtCompound {
    let mut data = NbtCompound::new();
    data.insert("Book", book.to_nbt_tag());
    data.insert("Page", page);
    data
}

fn place_lectern(world: &Arc<World>) -> SharedBlockEntity {
    assert!(world.set_block(
        POS,
        vanilla_blocks::LECTERN.default_state(),
        UpdateFlags::UPDATE_ALL
    ));
    world.get_block_entity(POS).expect("lectern should exist")
}

fn stored_pages(block_entity: &SharedBlockEntity) -> WrittenBookContent {
    block_entity
        .downcast_ref::<LecternBlockEntity>()
        .expect("lectern implementation")
        .book()
        .get(WRITTEN_BOOK_CONTENT)
        .expect("written book content")
        .clone()
}

fn storage_book() -> ItemStack {
    unresolved_book(vec![nbt_page(
        NbtSource::Storage("vecta:lectern_load".into()),
        "message",
    )])
}

fn set_storage(server: &Server, world: &World, message: &str) {
    let mut data = NbtCompound::new();
    data.insert("message", message);
    server
        .command_storage
        .get(world.domain())
        .expect("domain storage")
        .set(Identifier::new_static("vecta", "lectern_load"), data);
}

#[test]
fn live_load_uses_block_world_center_and_no_command_actor_without_container_lock() {
    with_world(|server, world, player| {
        player
            .inventory
            .lock()
            .set_selected_item(ItemStack::new(&vanilla_items::DIAMOND));
        let block_entity = place_lectern(world);
        let lectern = block_entity
            .downcast_ref::<LecternBlockEntity>()
            .expect("lectern");
        lectern.set_book(ItemStack::new(&vanilla_items::WRITABLE_BOOK));
        set_storage(server, world, "stored");
        let scoreboard = server
            .scoreboards
            .get(world.domain())
            .expect("domain scoreboard");
        let objective = scoreboard.add_objective("load_points").expect("objective");
        scoreboard
            .set_score(&ScoreHolder::new("*"), &objective, 7)
            .expect("wildcard score");
        scoreboard
            .set_score(&ScoreHolder::new(player.scoreboard_name()), &objective, 99)
            .expect("player score");
        let pages = vec![
            nbt_page(NbtSource::Entity("@s".into()), "SelectedItem.id"),
            Filterable::pass_through(TextComponent {
                content: Content::Resolvable(Resolvable::Entity {
                    selector: "@p[distance=..4]".into(),
                    separator: None,
                }),
                ..Default::default()
            }),
            Filterable::pass_through(TextComponent {
                content: Content::Resolvable(Resolvable::Scoreboard {
                    selector: "*".into(),
                    objective: "load_points".into(),
                }),
                ..Default::default()
            }),
            nbt_page(NbtSource::Storage("vecta:lectern_load".into()), "message"),
            nbt_page(NbtSource::Block("~ ~ ~".into()), "Book.id"),
        ];
        let source = CommandSource::new(
            CommandSender::Player(Arc::clone(player)),
            Arc::clone(server),
        );
        let context = BlockEntityLoadContext::for_source(&source).expect("live read context");
        block_entity
            .load_with_owned_components(&data(unresolved_book(pages), 999), &context)
            .expect("live load");
        let written = stored_pages(&block_entity);
        assert!(written.resolved());
        let actual: Vec<_> = written
            .pages()
            .iter()
            .map(|page| page.raw().to_string())
            .collect();
        assert_eq!(
            actual,
            [
                "",
                player.gameprofile.name(),
                "7",
                "\"stored\"",
                "\"minecraft:writable_book\""
            ]
        );
        assert_eq!(lectern.page_count(), 5);
        assert_eq!(lectern.page(), 4);
    });
}

#[test]
fn initial_factory_load_does_not_resolve_books_before_level_binding() {
    use vecta_registry::vanilla_block_entity_types;
    with_world(|server, world, player| {
        set_storage(server, world, "should not be read");
        let nbt = data(storage_book(), 50);
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed = read_compound(&mut Cursor::new(bytes.as_slice())).expect("borrowed NBT");
        let state = vanilla_blocks::LECTERN.default_state();
        let borrowed_entity = BLOCK_ENTITIES.create_and_load_or_unimplemented(
            &vanilla_block_entity_types::LECTERN,
            Arc::downgrade(world),
            POS,
            state,
            &borrowed,
        );
        let owned_entity = BLOCK_ENTITIES.create_and_load_owned_or_unimplemented(
            &vanilla_block_entity_types::LECTERN,
            Arc::downgrade(world),
            POS,
            state,
            nbt.clone(),
        );
        let source = CommandSource::new(
            CommandSender::Player(Arc::clone(player)),
            Arc::clone(server),
        );
        let context = BlockEntityLoadContext::for_source(&source).expect("live context");
        for entity in [borrowed_entity, owned_entity] {
            let before = stored_pages(&entity);
            assert!(!before.resolved());
            assert_eq!(
                before.pages()[0].raw(),
                storage_book()
                    .get(WRITTEN_BOOK_CONTENT)
                    .expect("source content")
                    .pages()[0]
                    .raw()
            );
            entity
                .load_with_owned_components(&nbt, &context)
                .expect("attached reload");
            assert!(stored_pages(&entity).resolved());
        }
    });
}

#[test]
fn resolved_book_does_not_read_changed_storage_on_reload() {
    with_world(|server, world, player| {
        let entity = place_lectern(world);
        let source = CommandSource::new(
            CommandSender::Player(Arc::clone(player)),
            Arc::clone(server),
        );
        let context = BlockEntityLoadContext::for_source(&source).expect("live context");
        set_storage(server, world, "first");
        entity
            .load_with_owned_components(&data(storage_book(), 0), &context)
            .expect("first load");
        let saved = entity.save_without_metadata();
        set_storage(server, world, "second");
        entity
            .load_with_owned_components(&saved, &context)
            .expect("resolved reload");
        assert_eq!(
            stored_pages(&entity).pages()[0].raw().to_string(),
            "\"first\""
        );
    });
}

#[test]
fn multiple_score_targets_preserve_original_page_on_resolution_failure() {
    with_world(|server, world, player| {
        let (other, _) = test_player_with_packets(server, Arc::clone(world), "OtherReader", 2);
        assert!(server.online_players.insert(Arc::clone(&other)));
        assert!(world.add_player(Arc::clone(&other), ResetReason::InitialJoin));
        let _ = other.mark_joined_world();
        let entity = place_lectern(world);
        let source = CommandSource::new(
            CommandSender::Player(Arc::clone(player)),
            Arc::clone(server),
        );
        let context = BlockEntityLoadContext::for_source(&source).expect("live context");
        let book = unresolved_book(vec![Filterable::pass_through(TextComponent {
            content: Content::Resolvable(Resolvable::Scoreboard {
                selector: "@a".into(),
                objective: "load_points".into(),
            }),
            ..Default::default()
        })]);
        let original = book
            .get(WRITTEN_BOOK_CONTENT)
            .expect("source content")
            .clone();
        entity
            .load_with_owned_components(&data(book, 0), &context)
            .expect("load with a runtime resolution error");
        assert_eq!(stored_pages(&entity), original.mark_resolved());
    });
}

#[test]
fn setblock_fill_and_execute_store_pass_live_load_context() {
    with_world(|server, world, _player| {
        set_storage(server, world, "command");
        let payload = to_canonical_snbt(&NbtTag::Compound(data(storage_book(), 0))).expect("SNBT");
        let run = |command: &str| {
            let source = CommandSource::new(CommandSender::Console, Arc::clone(server))
                .with_world(Arc::clone(world))
                .expect("target world");
            run_command(server, source, command);
        };
        for (command, pos) in [
            (
                format!("setblock 20 65 20 minecraft:lectern[has_book=true]{payload}"),
                POS,
            ),
            (
                format!("fill 21 65 20 21 65 20 minecraft:lectern[has_book=true]{payload}"),
                POS.east(),
            ),
        ] {
            run(&command);
            let entity = world.get_block_entity(pos).expect("command placed lectern");
            assert!(stored_pages(&entity).resolved(), "{command}");
            assert_eq!(
                stored_pages(&entity).pages()[0].raw().to_string(),
                "\"command\""
            );
        }
        let entity = world.get_block_entity(POS).expect("lectern");
        entity
            .downcast_ref::<LecternBlockEntity>()
            .expect("lectern implementation")
            .set_book(storage_book());
        assert!(!stored_pages(&entity).resolved());
        advance_test_game_time_to(server.overworld(), 10);
        run("execute store result block 20 65 20 Page int 1 run time query gametime");
        assert!(stored_pages(&entity).resolved());
        assert_eq!(
            stored_pages(&entity).pages()[0].raw().to_string(),
            "\"command\""
        );
    });
}
