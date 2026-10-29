use vecta_registry::vanilla_entities;

use super::{Player, PlayerGameModeState};
use vecta_utils::types::GameType;

#[test]
fn invalid_attack_targets_include_xp_orbs() {
    assert!(Player::is_invalid_attack_target(
        1,
        1,
        &vanilla_entities::PLAYER
    ));
    assert!(Player::is_invalid_attack_target(
        1,
        2,
        &vanilla_entities::ITEM
    ));
    assert!(Player::is_invalid_attack_target(
        1,
        2,
        &vanilla_entities::EXPERIENCE_ORB
    ));
    assert!(!Player::is_invalid_attack_target(
        1,
        2,
        &vanilla_entities::PIG
    ));
}

#[test]
fn changing_current_records_previous_mode() {
    let mut state = PlayerGameModeState::new(GameType::Survival);

    assert!(state.change_current(GameType::Creative));

    assert_eq!(state.current(), GameType::Creative);
    assert_eq!(state.previous(), Some(GameType::Survival));
}

#[test]
fn setting_same_mode_keeps_previous_mode() {
    let mut state = PlayerGameModeState::new(GameType::Survival);
    state.change_current(GameType::Creative);

    assert!(!state.change_current(GameType::Creative));

    assert_eq!(state.current(), GameType::Creative);
    assert_eq!(state.previous(), Some(GameType::Survival));
}

#[test]
fn persistent_restore_sets_current_and_previous() {
    let mut state = PlayerGameModeState::new(GameType::Survival);

    state.set_pair(GameType::Adventure, Some(GameType::Creative));

    assert_eq!(state.current(), GameType::Adventure);
    assert_eq!(state.previous(), Some(GameType::Creative));
}

#[test]
fn initial_state_has_no_previous_mode() {
    let state = PlayerGameModeState::new(GameType::Survival);

    assert_eq!(state.current(), GameType::Survival);
    assert_eq!(state.previous(), None);
}

mod pick_item {
    use std::sync::Arc;

    use glam::DVec3;
    use text_components::TextComponent;

    use crate::bootstrap::init_globals;
    use crate::entity::entities::{
        EndCrystalEntity, GlowItemFrameEntity, ItemFrameEntity, LeashFenceKnotEntity, MarkerEntity,
        PigEntity,
    };
    use crate::entity::{Entity, SharedEntity};
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
}
