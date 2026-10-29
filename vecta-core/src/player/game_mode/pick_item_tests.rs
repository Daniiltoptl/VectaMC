use std::sync::Arc;

use glam::DVec3;
use text_components::TextComponent;

use crate::bootstrap::init_globals;
use crate::entity::entities::{
    EndCrystalEntity, GlowItemFrameEntity, ItemFrameEntity, LeashFenceKnotEntity, MarkerEntity,
    PigEntity,
};
use crate::entity::{Entity, EntityVisibility, SharedEntity};
use crate::inventory::container::Container as _;
use crate::player::Player;
use crate::player::player_inventory::PlayerInventory;
use crate::test_support::{
    TestPlayerBuilder, TestWorld, fresh_test_world, insert_ready_full_chunk,
};
use crate::world::World;
use vecta_protocol::packets::game::SPickItemFromEntity;
use vecta_registry::data_components::vanilla_components::CUSTOM_NAME;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::{vanilla_entities, vanilla_items};
use vecta_utils::types::GameType;
use vecta_utils::{BlockPos, ChunkPos, Direction};

const PLAYER_POSITION: DVec3 = DVec3::new(0.5, 64.0, 0.5);

fn pick_test_player(name: &'static str, game_mode: GameType) -> (Arc<Player>, TestWorld) {
    init_globals();
    let fixture = fresh_test_world(name);
    insert_ready_full_chunk(&fixture.world, ChunkPos::new(0, 0));
    let player = TestPlayerBuilder::new(Arc::clone(&fixture.world), name, 1).build();
    player.set_client_loaded(true);
    // `set_game_mode` notifies the client and needs a live server handle, which
    // a detached test player does not have.
    player.restore_game_modes(game_mode, None);
    player.base().set_position_local(PLAYER_POSITION);
    (player, fixture)
}

fn add_entity(world: &Arc<World>, entity: SharedEntity) -> i32 {
    let id = entity.id();
    world
        .try_add_entity(entity)
        .expect("test entity should register in a loaded chunk");
    id
}

fn pig(world: &Arc<World>, id: i32, position: DVec3) -> SharedEntity {
    Arc::new(PigEntity::new(
        &vanilla_entities::PIG,
        id,
        position,
        Arc::downgrade(world),
    ))
}

fn held_item(player: &Player, slot: u8) -> ItemStack {
    player.inventory.lock().get_item(usize::from(slot)).clone()
}

fn selected_slot(player: &Player) -> u8 {
    player.inventory.lock().get_selected_slot()
}

#[test]
fn picking_a_mob_in_survival_moves_its_spawn_egg_into_the_hotbar() {
    let (player, fixture) = pick_test_player("PickMobSurvival", GameType::Survival);
    let entity_id = add_entity(
        &fixture.world,
        pig(&fixture.world, 2, DVec3::new(0.5, 64.0, 1.5)),
    );

    player
        .inventory
        .lock()
        .set_item(12, ItemStack::new(&vanilla_items::PIG_SPAWN_EGG));

    player.handle_pick_item_from_entity(SPickItemFromEntity {
        id: entity_id,
        include_data: false,
    });

    assert_eq!(selected_slot(&player), 0);
    assert!(held_item(&player, 0).is(&vanilla_items::PIG_SPAWN_EGG));
    assert!(held_item(&player, 12).is_empty());
}

#[test]
fn picking_a_mob_in_creative_adds_a_spawn_egg_when_missing() {
    let (player, fixture) = pick_test_player("PickMobCreative", GameType::Creative);
    let entity_id = add_entity(
        &fixture.world,
        pig(&fixture.world, 2, DVec3::new(0.5, 64.0, 1.5)),
    );

    player.handle_pick_item_from_entity(SPickItemFromEntity {
        id: entity_id,
        include_data: false,
    });

    assert_eq!(selected_slot(&player), 0);
    assert!(held_item(&player, 0).is(&vanilla_items::PIG_SPAWN_EGG));
}

#[test]
fn picking_a_mob_in_survival_without_the_egg_creates_nothing() {
    let (player, fixture) = pick_test_player("PickMobNoEgg", GameType::Survival);
    let entity_id = add_entity(
        &fixture.world,
        pig(&fixture.world, 2, DVec3::new(0.5, 64.0, 1.5)),
    );

    player.handle_pick_item_from_entity(SPickItemFromEntity {
        id: entity_id,
        include_data: false,
    });

    let inventory = player.inventory.lock();
    assert!(inventory.get_item(0).is_empty());
    assert_eq!(inventory.get_selected_slot(), 0);
}

#[test]
fn picking_a_filled_item_frame_returns_the_framed_stack() {
    let (player, fixture) = pick_test_player("PickFilledFrame", GameType::Creative);
    let frame = Arc::new(ItemFrameEntity::new_attached(
        &vanilla_entities::ITEM_FRAME,
        2,
        BlockPos::new(0, 64, 1),
        Direction::South,
        Arc::downgrade(&fixture.world),
    ));
    let mut framed = ItemStack::new(&vanilla_items::DIAMOND);
    framed.set(CUSTOM_NAME, TextComponent::plain("Framed Gem"));
    frame.set_item(framed);
    let entity_id = add_entity(&fixture.world, frame);

    player.handle_pick_item_from_entity(SPickItemFromEntity {
        id: entity_id,
        include_data: false,
    });

    let picked = held_item(&player, selected_slot(&player));
    assert!(picked.is(&vanilla_items::DIAMOND));
    assert_eq!(picked.count(), 1);
    assert_eq!(
        picked.get(CUSTOM_NAME),
        Some(&TextComponent::plain("Framed Gem")),
        "the framed stack's components must survive the pick"
    );
}

#[test]
fn picking_an_empty_item_frame_returns_the_frame_with_its_custom_name() {
    let (player, fixture) = pick_test_player("PickEmptyFrame", GameType::Creative);
    let frame = Arc::new(ItemFrameEntity::new_attached(
        &vanilla_entities::ITEM_FRAME,
        2,
        BlockPos::new(0, 64, 1),
        Direction::South,
        Arc::downgrade(&fixture.world),
    ));
    frame.set_custom_name(Some(TextComponent::plain("Wall Trophy")));
    let entity_id = add_entity(&fixture.world, frame);

    player.handle_pick_item_from_entity(SPickItemFromEntity {
        id: entity_id,
        include_data: false,
    });

    let picked = held_item(&player, selected_slot(&player));
    assert!(picked.is(&vanilla_items::ITEM_FRAME));
    assert_eq!(
        picked.get(CUSTOM_NAME),
        Some(&TextComponent::plain("Wall Trophy"))
    );
}

#[test]
fn picking_an_empty_glow_item_frame_returns_the_glow_variant() {
    let (player, fixture) = pick_test_player("PickGlowFrame", GameType::Creative);
    let frame = Arc::new(GlowItemFrameEntity::new(
        &vanilla_entities::GLOW_ITEM_FRAME,
        2,
        DVec3::new(0.5, 64.5, 1.03),
        Arc::downgrade(&fixture.world),
    ));
    let entity_id = add_entity(&fixture.world, frame);

    player.handle_pick_item_from_entity(SPickItemFromEntity {
        id: entity_id,
        include_data: false,
    });

    assert!(held_item(&player, selected_slot(&player)).is(&vanilla_items::GLOW_ITEM_FRAME));
}

#[test]
fn picking_a_leash_knot_returns_a_lead() {
    let (player, fixture) = pick_test_player("PickLeashKnot", GameType::Creative);
    let knot = Arc::new(LeashFenceKnotEntity::new_attached(
        &vanilla_entities::LEASH_KNOT,
        2,
        BlockPos::new(0, 64, 1),
        Arc::downgrade(&fixture.world),
    ));
    let entity_id = add_entity(&fixture.world, knot);

    player.handle_pick_item_from_entity(SPickItemFromEntity {
        id: entity_id,
        include_data: false,
    });

    assert!(held_item(&player, selected_slot(&player)).is(&vanilla_items::LEAD));
}

#[test]
fn picking_an_end_crystal_returns_an_end_crystal() {
    let (player, fixture) = pick_test_player("PickEndCrystal", GameType::Creative);
    let crystal = Arc::new(EndCrystalEntity::new(
        &vanilla_entities::END_CRYSTAL,
        2,
        DVec3::new(0.5, 64.5, 1.0),
        Arc::downgrade(&fixture.world),
    ));
    let entity_id = add_entity(&fixture.world, crystal);

    player.handle_pick_item_from_entity(SPickItemFromEntity {
        id: entity_id,
        include_data: false,
    });

    assert!(held_item(&player, selected_slot(&player)).is(&vanilla_items::END_CRYSTAL));
}

#[test]
fn picking_an_entity_without_a_pick_result_creates_nothing() {
    let (player, fixture) = pick_test_player("PickNoResult", GameType::Creative);
    let marker = Arc::new(MarkerEntity::new(
        &vanilla_entities::MARKER,
        2,
        DVec3::new(0.5, 64.0, 1.5),
        Arc::downgrade(&fixture.world),
    ));
    let entity_id = add_entity(&fixture.world, marker);

    player.handle_pick_item_from_entity(SPickItemFromEntity {
        id: entity_id,
        include_data: false,
    });

    assert!(held_item(&player, 0).is_empty());
    assert_eq!(selected_slot(&player), 0);
}

#[test]
fn picking_an_unknown_entity_id_creates_nothing() {
    let (player, _fixture) = pick_test_player("PickUnknownId", GameType::Creative);

    player.handle_pick_item_from_entity(SPickItemFromEntity {
        id: 999,
        include_data: false,
    });

    assert!(held_item(&player, 0).is_empty());
}

#[test]
fn picking_an_entity_in_a_hidden_chunk_creates_nothing() {
    let (player, fixture) = pick_test_player("PickHidden", GameType::Creative);
    let entity_id = add_entity(
        &fixture.world,
        pig(&fixture.world, 2, DVec3::new(0.5, 64.0, 1.5)),
    );
    fixture
        .world
        .update_entity_chunk_visibility(ChunkPos::new(0, 0), EntityVisibility::Hidden);
    assert!(fixture.world.get_entity_by_id(entity_id).is_some());
    assert!(
        fixture
            .world
            .get_accessible_entity_by_id(entity_id)
            .is_none()
    );

    player.handle_pick_item_from_entity(SPickItemFromEntity {
        id: entity_id,
        include_data: false,
    });

    assert!(held_item(&player, 0).is_empty());
}

#[test]
fn entity_pick_range_uses_attributes_and_excludes_the_exact_boundary() {
    use crate::entity::{LivingEntity as _, RemovalReason};
    use vecta_registry::vanilla_attributes;
    use vecta_utils::WorldAabb;

    let (player, fixture) = pick_test_player("PickRange", GameType::Survival);
    let target = pig(&fixture.world, 2, PLAYER_POSITION);
    target
        .base()
        .set_bounding_box(WorldAabb::new(5.5, 64.0, 0.0, 6.5, 67.0, 1.0));
    player
        .attributes()
        .lock()
        .set_base_value(vanilla_attributes::ENTITY_INTERACTION_RANGE, 2.0);

    assert!(!player.is_entity_within_interaction_range(target.as_ref(), 3.0));
    player
        .attributes()
        .lock()
        .set_base_value(vanilla_attributes::ENTITY_INTERACTION_RANGE, 2.01);
    assert!(player.is_entity_within_interaction_range(target.as_ref(), 3.0));
    target.set_removed(RemovalReason::Discarded);
    assert!(!player.is_entity_within_interaction_range(target.as_ref(), 3.0));
}

#[test]
fn creative_pick_relocates_the_displaced_stack_and_handles_a_full_inventory() {
    for full in [false, true] {
        let (player, _fixture) = pick_test_player("PickRelocate", GameType::Creative);
        {
            let mut inventory = player.inventory.lock();
            for slot in 0..if full {
                PlayerInventory::INVENTORY_SIZE
            } else {
                9
            } {
                inventory.set_item(slot, ItemStack::with_count(&vanilla_items::STONE, 17));
            }
            inventory.set_selected_slot(4);
        }

        player.try_pick_item(ItemStack::new(&vanilla_items::DIAMOND));

        assert_eq!(selected_slot(&player), 4);
        assert!(held_item(&player, 4).is(&vanilla_items::DIAMOND));
        let inventory = player.inventory.lock();
        let stone_count: i32 = inventory
            .get_items()
            .iter()
            .filter(|stack| stack.is(&vanilla_items::STONE))
            .map(ItemStack::count)
            .sum();
        assert_eq!(stone_count, if full { 35 * 17 } else { 9 * 17 });
        if !full {
            assert_eq!(inventory.get_item(9).count(), 17);
        }
    }
}

#[test]
fn picking_an_entity_out_of_reach_creates_nothing() {
    let (player, fixture) = pick_test_player("PickOutOfReach", GameType::Creative);
    let entity_id = add_entity(
        &fixture.world,
        pig(&fixture.world, 2, DVec3::new(0.5, 64.0, 10.5)),
    );

    player.handle_pick_item_from_entity(SPickItemFromEntity {
        id: entity_id,
        include_data: false,
    });

    assert!(held_item(&player, 0).is_empty());
}

#[test]
fn picking_an_existing_stack_selects_its_hotbar_slot_instead_of_moving_it() {
    let (player, fixture) = pick_test_player("PickExisting", GameType::Survival);
    let entity_id = add_entity(
        &fixture.world,
        pig(&fixture.world, 2, DVec3::new(0.5, 64.0, 1.5)),
    );

    player
        .inventory
        .lock()
        .set_item(4, ItemStack::new(&vanilla_items::PIG_SPAWN_EGG));

    player.handle_pick_item_from_entity(SPickItemFromEntity {
        id: entity_id,
        include_data: false,
    });

    assert_eq!(selected_slot(&player), 4);
    assert!(held_item(&player, 4).is(&vanilla_items::PIG_SPAWN_EGG));
    assert!(PlayerInventory::is_hotbar_slot(4));
}
