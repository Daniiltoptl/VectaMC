use std::sync::Arc;

use glam::DVec3;
use simdnbt::owned::{NbtCompound, NbtTag};
use text_components::TextComponent;
use vecta_registry::data_components::components::{CustomData, EntityData};
use vecta_registry::data_components::vanilla_components::{CUSTOM_NAME, ENTITY_DATA};
use vecta_registry::item_stack::ItemStack;
use vecta_registry::{vanilla_blocks, vanilla_entities, vanilla_items};
use vecta_utils::Direction;
use vecta_utils::types::{GameType, InteractionHand};

use super::{FRAME_POS, Fixture, with_borrowed_compound};
use crate::behavior::InteractionResult;
use crate::entity::entities::ItemEntity;
use crate::entity::{Entity as _, ItemFrame as _, SharedEntity};
use crate::test_support::dropped_items;
use vecta_utils::Downcast as _;

#[test]
fn entity_data_is_applied_before_placement_survival_checks() {
    vecta_registry::init_vanilla_registry();
    for (item, entity_type) in [
        (&*vanilla_items::ITEM_FRAME, &vanilla_entities::ITEM_FRAME),
        (
            &*vanilla_items::GLOW_ITEM_FRAME,
            &vanilla_entities::GLOW_ITEM_FRAME,
        ),
    ] {
        let fixture = Fixture::new("review_frame_entity_data");
        let mut payload = NbtCompound::new();
        payload.insert("Fixed", 1_i8);
        payload.insert("Invisible", 1_i8);
        payload.insert("Facing", Direction::East.get_3d_data_value() as i8);
        let mut stack = ItemStack::new(item);
        stack.set(
            ENTITY_DATA,
            EntityData::new(
                entity_type,
                CustomData::try_from_compound(payload).expect("valid frame payload"),
            ),
        );
        fixture
            .player
            .inventory
            .lock()
            .set_item_in_hand(InteractionHand::OffHand, stack);
        assert_eq!(
            fixture.use_on(
                FRAME_POS.relative(Direction::North),
                Direction::South,
                InteractionHand::OffHand
            ),
            InteractionResult::Success
        );
        let frames = fixture.frames_in_world();
        assert_eq!(frames.len(), 1);
        assert!(frames[0].is_invisible());
        assert_eq!(
            frames[0]
                .as_item_frame()
                .expect("frame capability")
                .direction(),
            Direction::East
        );
        assert_eq!(frames[0].nbt_for_data_compare().byte("Fixed"), Some(1));
    }
}

#[test]
fn named_regular_and_glow_frames_keep_their_name_in_drops() {
    for glow in [false, true] {
        let fixture = Fixture::new("review_named_frame_drop");
        let frame: SharedEntity = if glow {
            Arc::new(fixture.glow_frame())
        } else {
            Arc::new(fixture.frame())
        };
        let name = TextComponent::plain("Named frame");
        frame.set_custom_name(Some(name.clone()));
        frame
            .as_block_attached_entity()
            .expect("attached frame")
            .drop_item(None);
        let drops = dropped_items(fixture.world(), FRAME_POS.0.as_dvec3() + DVec3::splat(0.5));
        assert_eq!(drops.len(), 1);
        let stack = drops[0]
            .downcast_ref::<ItemEntity>()
            .expect("dropped frame")
            .get_item();
        assert_eq!(stack.get(CUSTOM_NAME), Some(&name));
        assert!(stack.is(if glow {
            &vanilla_items::GLOW_ITEM_FRAME
        } else {
            &vanilla_items::ITEM_FRAME
        }));
    }
}

#[test]
fn signed_rotation_and_legacy_direction_ids_keep_java_semantics() {
    let fixture = Fixture::new("review_frame_numeric_nbt");
    for facing in [
        NbtTag::Byte(-1),
        NbtTag::Short(257),
        NbtTag::Int(257),
        NbtTag::Float(257.9),
        NbtTag::Float(-1.2),
        NbtTag::Double(-1.2),
    ] {
        let frame = fixture.frame();
        let mut nbt = NbtCompound::new();
        nbt.insert(
            "Item",
            ItemStack::new(&vanilla_items::DIAMOND).to_nbt_tag_ref(),
        );
        nbt.insert("ItemRotation", -1_i8);
        nbt.insert("Facing", facing);
        with_borrowed_compound(&nbt, |value| frame.load_additional(value));
        assert_eq!(frame.rotation(), -1);
        assert_eq!(frame.analog_output(), 0);
        assert_eq!(frame.direction(), Direction::Up);
    }
}

#[test]
fn loading_missing_fields_resets_rotation_and_drop_chance() {
    let fixture = Fixture::new("review_frame_reload_defaults");
    let frame = fixture.frame();
    let mut initial = NbtCompound::new();
    initial.insert("ItemRotation", 3_i8);
    initial.insert("ItemDropChance", 0.0_f32);
    with_borrowed_compound(&initial, |value| frame.load_additional(value));
    with_borrowed_compound(&NbtCompound::new(), |value| frame.load_additional(value));
    let saved = frame.nbt_for_data_compare();
    assert_eq!(saved.byte("ItemRotation"), Some(0));
    assert_eq!(saved.float("ItemDropChance"), Some(1.0));
}

#[test]
fn numeric_nbt_fields_use_tag_value_input_conversions() {
    let fixture = Fixture::new("review_frame_numeric_fields");
    let frame = fixture.frame();
    let mut nbt = NbtCompound::new();
    nbt.insert("ItemRotation", -1.2_f64);
    nbt.insert("ItemDropChance", 0.25_f64);
    nbt.insert("Invisible", 257_i32);
    nbt.insert("Fixed", 256_i32);
    with_borrowed_compound(&nbt, |value| frame.load_additional(value));
    assert_eq!(frame.rotation(), -2);
    assert!(frame.is_invisible());
    assert!(!frame.is_fixed());
    assert_eq!(
        frame.nbt_for_data_compare().float("ItemDropChance"),
        Some(0.25)
    );
}

#[test]
fn border_touching_and_distant_outside_frames_use_border_collision_shapes() {
    let fixture = Fixture::new("review_frame_border");
    fixture.backing_block();
    let frame = fixture.frame();
    fixture
        .world()
        .set_world_border_center(frame.position().x, frame.position().z)
        .expect("border center");
    fixture
        .world()
        .set_world_border_size(0.75)
        .expect("border size");
    assert!(
        frame.survives(),
        "touching the border does not intersect its shape"
    );
    fixture
        .world()
        .set_world_border_size(0.5)
        .expect("smaller border");
    assert!(
        frame.survives(),
        "border collision shapes round outward to whole blocks"
    );
    fixture
        .world()
        .set_world_border_center(7.0, frame.position().z)
        .expect("border beside frame");
    fixture
        .world()
        .set_world_border_size(2.0)
        .expect("border edge at x=8");
    assert!(
        !frame.survives(),
        "nearby border shape cuts through the frame"
    );
    fixture
        .world()
        .set_world_border_center(1000.0, 1000.0)
        .expect("distant border");
    assert!(
        frame.survives(),
        "vanilla only applies border collisions close to the entity"
    );
}

#[test]
fn placement_covers_all_faces_both_hands_variants_and_game_modes() {
    vecta_registry::init_vanilla_registry();
    for mode in [GameType::Survival, GameType::Creative] {
        for hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
            for direction in Direction::ALL {
                for item in [
                    &*vanilla_items::ITEM_FRAME,
                    &*vanilla_items::GLOW_ITEM_FRAME,
                ] {
                    let fixture = Fixture::new("review_frame_placement_matrix");
                    fixture.player.restore_game_modes(mode, None);
                    fixture.player.abilities.lock().update_for_game_mode(mode);
                    let clicked = FRAME_POS.relative(direction.opposite());
                    fixture.block(clicked, vanilla_blocks::STONE.default_state());
                    fixture.hold(hand, item, 2);
                    assert_eq!(
                        fixture.use_on(clicked, direction, hand),
                        InteractionResult::Success
                    );
                    assert_eq!(
                        fixture.held(hand).count(),
                        if mode == GameType::Creative { 2 } else { 1 }
                    );
                    let frames = fixture.frames_in_world();
                    assert_eq!(frames.len(), 1);
                    assert_eq!(
                        frames[0].as_item_frame().expect("frame").direction(),
                        direction
                    );
                    fixture.hold(hand, &vanilla_items::DIAMOND, 3);
                    assert_eq!(
                        frames[0].interact(&fixture.player, hand, DVec3::ZERO),
                        InteractionResult::Success
                    );
                    assert_eq!(
                        fixture.held(hand).count(),
                        if mode == GameType::Creative { 3 } else { 2 }
                    );
                }
            }
        }
    }
}

#[test]
fn same_facing_frames_cannot_overlap_even_across_variants() {
    let fixture = Fixture::new("review_frame_overlap");
    fixture.backing_block();
    let existing: SharedEntity = Arc::new(fixture.frame());
    fixture
        .world()
        .try_add_entity(existing)
        .expect("add existing frame");
    for item in [
        &*vanilla_items::ITEM_FRAME,
        &*vanilla_items::GLOW_ITEM_FRAME,
    ] {
        fixture.hold(InteractionHand::MainHand, item, 1);
        assert_eq!(
            fixture.use_on(
                FRAME_POS.relative(Direction::North),
                Direction::South,
                InteractionHand::MainHand
            ),
            InteractionResult::Consume
        );
        assert_eq!(fixture.held(InteractionHand::MainHand).count(), 1);
        assert_eq!(fixture.frames_in_world().len(), 1);
    }
}

#[test]
fn support_loss_waits_for_the_periodic_survival_check() {
    let fixture = Fixture::new("review_frame_tick_support");
    fixture.backing_block();
    let frame = Arc::new(fixture.frame());
    fixture
        .world()
        .try_add_entity(Arc::clone(&frame) as SharedEntity)
        .expect("add frame");
    fixture.block(
        FRAME_POS.relative(Direction::North),
        vanilla_blocks::AIR.default_state(),
    );
    for _ in 0..100 {
        Arc::clone(&frame).tick();
        assert!(!frame.is_removed());
    }
    Arc::clone(&frame).tick();
    assert!(frame.is_removed());
    assert_eq!(
        dropped_items(fixture.world(), FRAME_POS.0.as_dvec3() + DVec3::splat(0.5)).len(),
        1
    );
}
