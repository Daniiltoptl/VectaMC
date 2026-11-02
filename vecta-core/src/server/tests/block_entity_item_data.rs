use super::*;

use simdnbt::ToNbtTag as _;
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::blocks::properties::BlockStateProperties;
use vecta_registry::data_components::components::{BlockEntityData, CustomData};
use vecta_registry::data_components::vanilla_components::BLOCK_ENTITY_DATA;
use vecta_registry::vanilla_block_entity_types;
use vecta_utils::Direction;
use vecta_utils::types::{GameType, InteractionHand};

use crate::behavior::items::BlockItem;
use crate::behavior::{
    BlockHitResult, BlockPlaceContext, InteractionResult, ItemBehavior, UseOnContext,
};
use crate::bootstrap::init_globals;

fn lectern_item() -> ItemStack {
    let mut payload = NbtCompound::new();
    payload.insert(
        "Book",
        ItemStack::new(&vanilla_items::WRITABLE_BOOK).to_nbt_tag(),
    );
    let mut item = ItemStack::new(&vanilla_items::LECTERN);
    item.set(
        BLOCK_ENTITY_DATA,
        BlockEntityData::new(
            &vanilla_block_entity_types::LECTERN,
            CustomData::try_from_compound(payload).expect("valid lectern custom data"),
        ),
    );
    item
}

#[test]
fn protected_block_entity_item_data_requires_both_operator_and_creative() {
    init_globals();
    let fixture = fresh_test_world("protected_block_entity_data");
    let world = &fixture.world;
    insert_ready_full_chunk(world, ChunkPos::new(0, 0));
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    runtime.block_on(async {
        let server = test_server(Arc::clone(world), PermissionSubjectIndex::new())
            .await
            .expect("server");
        let player = test_player(&server, Arc::clone(world));
        player
            .try_set_position(DVec3::new(8.5, 64.0, 8.5))
            .expect("player position");
        for (index, (operator, mode, allowed)) in [
            (false, GameType::Survival, false),
            (false, GameType::Creative, false),
            (true, GameType::Survival, false),
            (true, GameType::Creative, true),
        ]
        .into_iter()
        .enumerate()
        {
            server.player_permission_states.write().set(
                player.uuid(),
                PermissionSubjectState::new(
                    if operator {
                        vec![OP_GROUP.to_owned()]
                    } else {
                        Vec::new()
                    },
                    PermissionSet::new(),
                ),
            );
            player.restore_game_modes(mode, None);
            player.abilities.lock().update_for_game_mode(mode);
            let ground = BlockPos::new(2 + i32::try_from(index).expect("small index"), 63, 2);
            assert!(world.set_block(
                ground,
                vanilla_blocks::STONE.default_state(),
                UpdateFlags::UPDATE_ALL
            ));
            player
                .inventory
                .lock()
                .set_item_in_hand(InteractionHand::MainHand, lectern_item());
            let hit = BlockHitResult {
                location: ground.above().get_center().into(),
                direction: Direction::Up,
                block_pos: ground,
                miss: false,
                inside: false,
                world_border_hit: false,
            };
            let mut context = UseOnContext::new(
                &player,
                InteractionHand::MainHand,
                hit,
                world,
                Arc::clone(&player.inventory),
            );
            assert_eq!(
                BlockItem::new(&vanilla_blocks::LECTERN).use_on(&mut context),
                InteractionResult::Success
            );
            let placed = world
                .get_block_entity(ground.above())
                .expect("placed lectern");
            assert_eq!(
                placed.save_custom_only().get("Book").is_some(),
                allowed,
                "operator={operator}, mode={mode:?}"
            );
            assert_eq!(
                world
                    .get_block_state(ground.above())
                    .get_value(&BlockStateProperties::HAS_BOOK),
                allowed
            );
        }
    });
}

#[test]
fn playerless_placement_cannot_apply_protected_block_entity_data() {
    init_globals();
    let fixture = fresh_test_world("playerless_protected_block_entity_data");
    let world = &fixture.world;
    let pos = BlockPos::new(2, 64, 2);
    insert_ready_full_chunk(world, ChunkPos::new(0, 0));
    assert!(world.set_block(
        pos.below(),
        vanilla_blocks::STONE.default_state(),
        UpdateFlags::UPDATE_ALL
    ));
    let mut item = lectern_item();
    let context =
        BlockPlaceContext::directional(world, pos, Direction::North, &mut item, Direction::Up);
    assert_eq!(
        BlockItem::new(&vanilla_blocks::LECTERN).place(context),
        InteractionResult::Success
    );
    assert!(
        world
            .get_block_entity(pos)
            .expect("placed lectern")
            .save_custom_only()
            .get("Book")
            .is_none()
    );
}
