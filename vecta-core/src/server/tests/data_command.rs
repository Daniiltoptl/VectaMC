//! `/data` behaviour against a live server, world and player.

use std::iter;

use crate::chunk::Chunk;
use crate::chunk::status::ChunkStatus;
use simdnbt::ToNbtTag as _;
use simdnbt::owned::NbtList;
use text_components::content::NbtSource;
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::blocks::properties::{BlockStateProperties, Direction};
use vecta_registry::data_components::components::WrittenBookContent;
use vecta_registry::data_components::vanilla_components::WRITTEN_BOOK_CONTENT;
use vecta_utils::types::InteractionHand;
use vecta_utils::{Downcast as _, Identifier, nbt::to_canonical_snbt};

use super::book_resolution::{nbt_page, unresolved_book};
use super::*;

use crate::behavior::{BLOCK_BEHAVIORS, BlockHitResult, InventoryAccess};
use crate::block_entity::entities::{BarrelBlockEntity, ComparatorBlockEntity, LecternBlockEntity};
use crate::block_entity::{BlockEntity as _, BlockEntityLoadContext, SharedBlockEntity};
use crate::bootstrap::init_globals;
use crate::inventory::lock::ContainerLockGuard;

const POS: BlockPos = BlockPos::new(30, 65, 30);
const OTHER: BlockPos = BlockPos::new(32, 65, 30);

/// Builds one container entry in the shape `BarrelBlockEntity::save_additional`
/// writes, so a command payload round-trips through the real loader.
fn item_entry(slot: i8, item: &ItemStack) -> NbtCompound {
    let NbtTag::Compound(mut entry) = item.clone().to_nbt_tag() else {
        panic!("an item stack encodes as a compound");
    };
    entry.insert("Slot", slot);
    entry
}

fn items_payload(entries: Vec<NbtCompound>) -> String {
    let mut payload = NbtCompound::new();
    payload.insert("Items", NbtList::Compound(entries));
    to_canonical_snbt(&NbtTag::Compound(payload)).expect("payload renders as SNBT")
}

pub(super) fn with_world(
    key: &'static str,
    test: impl FnOnce(&Arc<Server>, &Arc<World>, &Arc<Player>, &Arc<SyncMutex<Vec<EncodedPacket>>>),
) {
    init_globals();
    let fixture = fresh_test_world(key);
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime");
    runtime.block_on(async {
        let world = &fixture.world;
        let domain = ResolvedDomainConfig {
            name: world.domain().to_owned(),
            default_world: world.key.clone(),
            worlds: vec![world.key.clone()],
        };
        // `/data` is a game-master command, so the operator contract must pass.
        let uuid = Uuid::new_v4();
        let mut permissions = PermissionSubjectIndex::new();
        permissions.set(
            uuid,
            PermissionSubjectState::new(vec![OP_GROUP.to_owned()], PermissionSet::new()),
        );
        let server = test_server_with_worlds_and_config(
            domain.name.clone(),
            slice::from_ref(&domain),
            slice::from_ref(world),
            permissions,
            test_runtime_config_with_max_players(1),
        )
        .await
        .expect("test server");
        for x in 0..=2 {
            for z in 0..=2 {
                insert_ready_full_chunk(world, ChunkPos::new(x, z));
            }
        }
        // Reserve a real id from the global allocator so spawned entities cannot
        // collide with the fixture player.
        let player_id = next_entity_id();
        let (player, packets) = test_player_with_uuid_and_packets(
            &server,
            Arc::clone(world),
            uuid,
            "Tester",
            player_id,
        );
        player.base().set_position_local(POS.get_center().into());
        assert!(server.online_players.insert(Arc::clone(&player)));
        assert!(world.add_player(Arc::clone(&player), ResetReason::InitialJoin));
        let _ = player.mark_joined_world();
        test(&server, world, &player, &packets);
    });
}

pub(super) fn source(server: &Arc<Server>, player: &Arc<Player>) -> CommandSource {
    CommandSource::new(
        CommandSender::Player(Arc::clone(player)),
        Arc::clone(server),
    )
}

/// Runs one command, treating a parse failure as `(false, 0)`.
///
/// `ResourceOrIdArgument` rejects unknown keys while parsing, so negative
/// compute tests observe the failure here instead of in the executor.
pub(super) fn try_run(server: &Arc<Server>, source: &CommandSource, command: &str) -> (bool, i32) {
    let parse_result = {
        let dispatcher = server.command_dispatcher.read();
        let parse = dispatcher.parse(command, source.clone());
        dispatcher.context_chain(parse).is_ok()
    };
    if !parse_result {
        return (false, 0);
    }
    run(server, source, command)
}

/// Runs one command and returns the reported `(success, result)`.
pub(super) fn run(server: &Server, source: &CommandSource, command: &str) -> (bool, i32) {
    let result = Arc::new(SyncMutex::new(None));
    let slot = Arc::clone(&result);
    let callback = CommandResultCallback::new(move |success, value| {
        *slot.lock() = Some((success, value));
    });
    run_command(server, source.clone().with_callback(callback), command);
    let reported = *result.lock();
    reported.unwrap_or_else(|| panic!("`{command}` should report a result"))
}

fn place_barrel(world: &Arc<World>, pos: BlockPos) -> SharedBlockEntity {
    assert!(world.set_block(
        pos,
        vanilla_blocks::BARREL.default_state(),
        UpdateFlags::UPDATE_ALL
    ));
    world.get_block_entity(pos).expect("barrel entity")
}

fn as_barrel(block_entity: &SharedBlockEntity) -> &BarrelBlockEntity {
    block_entity
        .downcast_ref::<BarrelBlockEntity>()
        .expect("barrel implementation")
}

fn barrel_items(block_entity: &SharedBlockEntity, slot: usize) -> ItemStack {
    let container = as_barrel(block_entity)
        .container_ref()
        .expect("barrel container");
    let id = container.container_id();
    let guard = ContainerLockGuard::lock_all(slice::from_ref(&container));
    guard
        .get(id)
        .expect("the barrel container is locked")
        .get_item(slot)
        .clone()
}

#[test]
fn data_get_merge_and_remove_operate_on_live_block_entity_state() {
    with_world(
        "data_get_merge_and_remove_operate_on_live_block_entity_state",
        |server, world, player, _packets| {
            let barrel = place_barrel(world, POS);
            let source = source(server, player);
            let payload = items_payload(vec![item_entry(
                0,
                &ItemStack::with_count(&vanilla_items::DIAMOND, 5),
            )]);

            assert_eq!(
                run(
                    server,
                    &source,
                    &format!("data merge block 30 65 30 {payload}")
                ),
                (true, 1)
            );
            assert_eq!(
                barrel_items(&barrel, 0),
                ItemStack::with_count(&vanilla_items::DIAMOND, 5),
                "the payload reaches the live container"
            );

            // `get` reports the vanilla result value: a scaled numeric tag.
            assert_eq!(
                run(server, &source, "data get block 30 65 30 Items[0].count 2"),
                (true, 10)
            );
            assert_eq!(
                run(server, &source, "data get block 30 65 30 Items"),
                (true, 1),
                "a collection reports its size"
            );

            assert_eq!(
                run(server, &source, "data remove block 30 65 30 Items"),
                (true, 1)
            );
            assert!(
                barrel_items(&barrel, 0).is_empty(),
                "`remove` deletes the selected tag from the live entity"
            );

            // Removing a path that does not resolve changes nothing, and the command
            // must say so instead of reporting success.
            let (success, _) = run(server, &source, "data remove block 30 65 30 Absent.path");
            assert!(
                !success,
                "vanilla reports `commands.data.merge.failed` instead of succeeding silently"
            );
        },
    );
}

#[test]
fn data_modify_moves_items_between_block_entities() {
    with_world(
        "data_modify_moves_items_between_block_entities",
        |server, world, player, _packets| {
            let source_barrel = place_barrel(world, POS);
            let target_barrel = place_barrel(world, OTHER);
            let source = source(server, player);
            assert_eq!(
                run(
                    server,
                    &source,
                    &format!(
                        "data merge block 30 65 30 {}",
                        items_payload(vec![item_entry(
                            3,
                            &ItemStack::with_count(&vanilla_items::EMERALD, 7),
                        )])
                    )
                ),
                (true, 1)
            );

            assert_eq!(
                run(
                    server,
                    &source,
                    "data modify block 32 65 30 Items append from block 30 65 30 Items[0]"
                ),
                (true, 1)
            );
            assert_eq!(
                barrel_items(&target_barrel, 3),
                ItemStack::with_count(&vanilla_items::EMERALD, 7),
                "`modify … append … from` copies the selected entry, including its slot"
            );
            assert_eq!(
                barrel_items(&source_barrel, 3),
                ItemStack::with_count(&vanilla_items::EMERALD, 7),
                "the source entity keeps its own copy"
            );

            assert_eq!(
                run(
                    server,
                    &source,
                    "data modify block 32 65 30 Items[0].count set value 9"
                ),
                (true, 1)
            );
            assert_eq!(
                barrel_items(&target_barrel, 3),
                ItemStack::with_count(&vanilla_items::EMERALD, 9)
            );

            // An out-of-range insertion is rejected without touching the contents.
            let (success, _) = run(
                server,
                &source,
                "data modify block 32 65 30 Items insert 99 from block 30 65 30 Items[0]",
            );
            assert!(!success, "an invalid list index must fail the command");
            assert_eq!(
                barrel_items(&target_barrel, 3),
                ItemStack::with_count(&vanilla_items::EMERALD, 9),
                "a rejected manipulation leaves the entity untouched"
            );
        },
    );
}

#[test]
fn data_merge_resolves_a_lectern_book_with_the_live_command_context() {
    with_world(
        "data_merge_resolves_a_lectern_book_with_the_live_command_context",
        |server, world, player, _packets| {
            assert!(world.set_block(
                POS,
                vanilla_blocks::LECTERN.default_state(),
                UpdateFlags::UPDATE_ALL
            ));
            let block_entity = world.get_block_entity(POS).expect("lectern entity");
            let lectern = block_entity
                .downcast_ref::<LecternBlockEntity>()
                .expect("lectern implementation");
            lectern.set_book(ItemStack::new(&vanilla_items::WRITABLE_BOOK));

            let mut message = NbtCompound::new();
            message.insert("message", "stored page");
            server
                .command_storage
                .get(world.domain())
                .expect("domain storage")
                .set(Identifier::new_static("vecta", "data_command"), message);

            let book = unresolved_book(vec![nbt_page(
                NbtSource::Storage("vecta:data_command".into()),
                "message",
            )]);
            let mut payload = NbtCompound::new();
            payload.insert("Book", book.to_nbt_tag());
            let payload = to_canonical_snbt(&NbtTag::Compound(payload)).expect("payload SNBT");

            let source = source(server, player);
            assert_eq!(
                run(
                    server,
                    &source,
                    &format!("data merge block 30 65 30 {payload}")
                ),
                (true, 1)
            );

            let written: WrittenBookContent = lectern
                .book()
                .get(WRITTEN_BOOK_CONTENT)
                .expect("written book content")
                .clone();
            assert!(
                written.resolved(),
                "`/data` loads through the live command context, so server-dependent text resolves"
            );
            assert_eq!(written.pages()[0].raw().to_string(), "\"stored page\"");

            // A later storage change must not be re-resolved by a plain reload.
            let mut message = NbtCompound::new();
            message.insert("message", "changed page");
            server
                .command_storage
                .get(world.domain())
                .expect("domain storage")
                .set(Identifier::new_static("vecta", "data_command"), message);
            block_entity
                .load_with_owned_components(
                    &block_entity.save_with_full_metadata(),
                    &BlockEntityLoadContext::detached(),
                )
                .expect("detached reload");
            assert_eq!(
                lectern
                    .book()
                    .get(WRITTEN_BOOK_CONTENT)
                    .expect("written book content")
                    .pages()[0]
                    .raw()
                    .to_string(),
                "\"stored page\"",
                "an already resolved book is not resolved again on reload"
            );
        },
    );
}

#[test]
fn data_merge_reaches_an_open_barrel_menu() {
    with_world(
        "data_merge_reaches_an_open_barrel_menu",
        |server, world, player, packets| {
            let barrel = place_barrel(world, POS);
            let behavior = BLOCK_BEHAVIORS.get_behavior(&vanilla_blocks::BARREL);
            let hit = BlockHitResult {
                location: POS.get_center().into(),
                direction: vecta_utils::Direction::Up,
                block_pos: POS,
                miss: false,
                inside: false,
                world_border_hit: false,
            };
            let mut inventory =
                InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);
            behavior.use_without_item(
                world.get_block_state(POS),
                world,
                POS,
                player,
                &hit,
                &mut inventory,
            );
            let open_container = as_barrel(&barrel)
                .container_ref()
                .expect("barrel container");
            assert!(
                player.has_container_open_for(&open_container),
                "the viewer's menu owns the live barrel container instead of a copy"
            );

            let before = packets.lock().len();
            let source = source(server, player);
            assert_eq!(
                run(
                    server,
                    &source,
                    &format!(
                        "data merge block 30 65 30 {}",
                        items_payload(vec![item_entry(
                            0,
                            &ItemStack::with_count(&vanilla_items::DIAMOND, 4),
                        )])
                    )
                ),
                (true, 1)
            );
            assert_eq!(
                barrel_items(&barrel, 0),
                ItemStack::with_count(&vanilla_items::DIAMOND, 4)
            );

            // Vanilla `ServerPlayer.tick` -> `containerMenu.broadcastChanges()`.
            player.broadcast_inventory_changes();
            assert!(
                packets.lock().len() > before,
                "the open viewer is sent the changed slot instead of keeping a stale copy"
            );
        },
    );
}

#[test]
fn data_merge_marks_the_chunk_unsaved() {
    with_world(
        "data_merge_marks_the_chunk_unsaved",
        |server, world, player, _packets| {
            let barrel = place_barrel(world, POS);
            let chunk_pos = ChunkPos::from_block_pos(POS);
            let is_dirty = || {
                world
                    .chunk_map
                    .with_chunk_at_status(chunk_pos, ChunkStatus::Full, Chunk::is_dirty)
                    .unwrap_or(false)
            };
            world
                .chunk_map
                .with_chunk_at_status(chunk_pos, ChunkStatus::Full, Chunk::clear_dirty);
            assert!(!is_dirty(), "the fixture starts from a saved chunk");

            let source = source(server, player);
            assert_eq!(
                run(
                    server,
                    &source,
                    &format!(
                        "data merge block 30 65 30 {}",
                        items_payload(vec![item_entry(
                            0,
                            &ItemStack::with_count(&vanilla_items::DIAMOND, 64),
                        )])
                    )
                ),
                (true, 1)
            );
            assert_eq!(
                barrel_items(&barrel, 0),
                ItemStack::with_count(&vanilla_items::DIAMOND, 64)
            );
            assert!(
                is_dirty(),
                "`BlockEntity.setChanged` marks the chunk unsaved after a `/data` write"
            );
        },
    );
}

#[test]
fn data_merge_updates_a_comparator_through_the_scheduled_tick() {
    with_world(
        "data_merge_updates_a_comparator_through_the_scheduled_tick",
        |server, world, player, _| {
            place_barrel(world, POS);
            let comparator_pos = POS.west();
            world.set_block(
                comparator_pos.below(),
                vanilla_blocks::STONE.default_state(),
                UpdateFlags::UPDATE_ALL,
            );
            let state = vanilla_blocks::COMPARATOR
                .default_state()
                .set_value(&BlockStateProperties::HORIZONTAL_FACING, Direction::East);
            assert!(world.set_block(comparator_pos, state, UpdateFlags::UPDATE_ALL));
            let comparator_entity = world.get_block_entity(comparator_pos).expect("comparator");
            let comparator = comparator_entity
                .downcast_ref::<ComparatorBlockEntity>()
                .expect("typed comparator");
            assert_eq!(comparator.output_signal(), 0);
            let source = source(server, player);
            let payload = items_payload(
                (0..27)
                    .map(|slot| {
                        item_entry(slot, &ItemStack::with_count(&vanilla_items::DIAMOND, 64))
                    })
                    .collect(),
            );
            assert_eq!(
                run(
                    server,
                    &source,
                    &format!("data merge block 30 65 30 {payload}")
                ),
                (true, 1)
            );
            assert_eq!(
                comparator.output_signal(),
                0,
                "output waits for the comparator tick"
            );
            assert!(world.has_scheduled_block_tick(comparator_pos, &vanilla_blocks::COMPARATOR));
            world
                .reconcile_active_scheduled_tick_chunks(iter::once(ChunkPos::from_block_pos(
                    comparator_pos,
                )))
                .expect("active chunk");
            let now = world.game_time();
            assert!(
                world
                    .begin_scheduled_tick_phase(now + 1, 64)
                    .ticks
                    .is_empty()
            );
            let due = world.begin_scheduled_tick_phase(now + 2, 64);
            assert_eq!(due.ticks.len(), 1);
            assert_eq!(due.ticks[0].pos, comparator_pos);
            let behavior = BLOCK_BEHAVIORS.get_behavior(&vanilla_blocks::COMPARATOR);
            behavior.tick(world.get_block_state(comparator_pos), world, comparator_pos);
            assert_eq!(comparator.output_signal(), 15);
            assert_eq!(
                run(server, &source, "data remove block 30 65 30 Items"),
                (true, 1)
            );
            assert!(world.has_scheduled_block_tick(comparator_pos, &vanilla_blocks::COMPARATOR));
            behavior.tick(world.get_block_state(comparator_pos), world, comparator_pos);
            assert_eq!(comparator.output_signal(), 0);
        },
    );
}

#[test]
fn data_rejects_invalid_targets_and_unchanged_merges() {
    with_world(
        "data_rejects_invalid_targets_and_unchanged_merges",
        |server, world, player, _packets| {
            let barrel = place_barrel(world, POS);
            assert!(world.set_block(
                OTHER,
                vanilla_blocks::STONE.default_state(),
                UpdateFlags::UPDATE_ALL
            ));
            let source = source(server, player);

            let (success, _) = run(server, &source, "data merge block 32 65 30 {lock:\"x\"}");
            assert!(
                !success,
                "a block without a block entity is rejected with `commands.data.block.invalid`"
            );

            assert_eq!(
                run(
                    server,
                    &source,
                    "data merge block 30 65 30 {lock:{items:\"minecraft:diamond\"}}"
                ),
                (true, 1)
            );
            assert!(as_barrel(&barrel).identity().is_locked());

            let (success, _) = run(
                server,
                &source,
                "data merge block 30 65 30 {lock:{items:\"minecraft:diamond\"}}",
            );
            assert!(
                !success,
                "merging identical data reports `commands.data.merge.failed`"
            );
            assert!(
                as_barrel(&barrel).identity().is_locked(),
                "a rejected merge keeps the previous state"
            );
        },
    );
}
