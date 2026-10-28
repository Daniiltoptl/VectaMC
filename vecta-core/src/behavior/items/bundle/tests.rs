use std::sync::Arc;

use glam::DVec3;
use vecta_registry::data_components::components::BundleContents;
use vecta_registry::item_stack_template::ItemStackTemplate;
use vecta_registry::stat::Stat;
use vecta_registry::{vanilla_damage_types, vanilla_items};
use vecta_utils::types::{GameType, InteractionHand};
use vecta_utils::{ChunkPos, Downcast as _, WorldAabb};

use super::*;
use crate::bootstrap::init_globals;
use crate::entity::damage::DamageSource;
use crate::entity::{SharedEntity, entities::ItemEntity, next_entity_id};
use crate::player::game_mode::use_item;
use crate::test_support::{
    TestPlayerBuilder, TestWorld, fresh_test_world, insert_ready_full_chunk,
};

fn fixture(name: &'static str) -> (TestWorld, Arc<Player>) {
    init_globals();
    let fixture = fresh_test_world(name);
    insert_ready_full_chunk(&fixture.world, ChunkPos::new(0, 0));
    let player =
        TestPlayerBuilder::new(Arc::clone(&fixture.world), "BundleReview", next_entity_id())
            .build();
    player.base().set_position_local(DVec3::new(0.5, 64.0, 0.5));
    player.set_rotation((0.0, 0.0));
    (fixture, player)
}

fn filled_bundle() -> ItemStack {
    let mut stack = ItemStack::new(&vanilla_items::BUNDLE);
    stack.set(
        BUNDLE_CONTENTS,
        BundleContents::new(vec![
            ItemStackTemplate::with_count(&vanilla_items::STONE, 3),
            ItemStackTemplate::with_count(&vanilla_items::DIAMOND, 2),
            ItemStackTemplate::new(&vanilla_items::DIRT),
        ]),
    );
    stack
}

fn drops(world: &Arc<World>) -> Vec<SharedEntity> {
    world.get_entities_in_aabb_matching(
        &WorldAabb::new(-2.0, 62.0, -2.0, 3.0, 68.0, 3.0),
        |entity| !entity.is_removed() && entity.downcast_ref::<ItemEntity>().is_some(),
    )
}

#[test]
fn bundle_review_hand_use_uses_the_player_drop_path() {
    for (name, hand, mode) in [
        (
            "bundle_review_main_survival",
            InteractionHand::MainHand,
            GameType::Survival,
        ),
        (
            "bundle_review_offhand_survival",
            InteractionHand::OffHand,
            GameType::Survival,
        ),
        (
            "bundle_review_main_creative",
            InteractionHand::MainHand,
            GameType::Creative,
        ),
        (
            "bundle_review_offhand_creative",
            InteractionHand::OffHand,
            GameType::Creative,
        ),
    ] {
        let (fixture, player) = fixture(name);
        player.restore_game_modes(mode, None);
        player
            .inventory
            .lock()
            .set_item_in_hand(hand, filled_bundle());
        assert_eq!(
            use_item(&player, &fixture.world, hand),
            InteractionResult::Success
        );
        let entity: SharedEntity = Arc::<Player>::clone(&player);
        let mut stack = player.inventory.lock().get_item_in_hand(hand).clone();
        BundleItem.on_use_tick(
            &fixture.world,
            LivingEntityRef::new(&entity).expect("test player should be living"),
            &mut stack,
            200,
        );

        let drops = drops(&fixture.world);
        assert_eq!(drops.len(), 1);
        let dropped = drops[0]
            .as_ref()
            .downcast_ref::<ItemEntity>()
            .expect("spawned entity should be an item");
        assert!(dropped.get_item().is(&vanilla_items::STONE));
        assert_eq!(dropped.get_thrower(), Some(player.gameprofile.id()));
        assert!(dropped.velocity().z > 0.27);
        assert_eq!(dropped.get_pickup_delay(), 40);
        assert!(player.stats().contains(&(
            Stat::new(&vanilla_stat_types::ITEM_DROPPED, &vanilla_items::STONE),
            3
        )));
        assert!(player.stats().contains(&(
            Stat::new(&vanilla_stat_types::ITEM_USED, &vanilla_items::BUNDLE),
            1
        )));
        assert_eq!(
            stack
                .get(BUNDLE_CONTENTS)
                .expect("stack should keep bundle contents")
                .items()
                .len(),
            2
        );
    }
}

#[test]
fn bundle_review_use_tick_preserves_the_throw_schedule_and_selected_item() {
    let (fixture, player) = fixture("bundle_review_schedule");
    let entity: SharedEntity = player;
    let mut stack = filled_bundle();
    let mut contents = stack
        .get(BUNDLE_CONTENTS)
        .expect("stack should keep bundle contents")
        .as_mutable();
    contents.toggle_selected_item(1);
    stack.set(
        BUNDLE_CONTENTS,
        contents
            .to_immutable()
            .expect("bundle contents should convert"),
    );

    for ticks in (188..=200).rev() {
        BundleItem.on_use_tick(
            &fixture.world,
            LivingEntityRef::new(&entity).expect("test player should be living"),
            &mut stack,
            ticks,
        );
        assert_eq!(
            drops(&fixture.world).len(),
            if ticks == 188 { 2 } else { 1 }
        );
    }
    let dropped = drops(&fixture.world);
    let mut items: Vec<_> = dropped
        .iter()
        .map(|entity| {
            entity
                .as_ref()
                .downcast_ref::<ItemEntity>()
                .expect("spawned entity should be an item")
                .get_item()
                .item()
        })
        .collect();
    items.sort_by_key(|item| item.key.to_string());
    assert_eq!(
        items,
        vec![&*vanilla_items::DIAMOND, &*vanilla_items::STONE]
    );
    assert_eq!(
        stack
            .get(BUNDLE_CONTENTS)
            .expect("stack should keep bundle contents")
            .items()[0]
            .item(),
        &*vanilla_items::DIRT
    );
}

#[test]
fn bundle_review_destroyed_item_releases_contents_without_hand_drop_state() {
    let (fixture, _) = fixture("bundle_review_destroyed");
    let bundle = fixture
        .world
        .spawn_item(DVec3::new(0.5, 64.0, 0.5), filled_bundle())
        .expect("bundle test setup should be valid");
    let source = DamageSource::environment(&vanilla_damage_types::GENERIC);

    assert!(bundle.hurt(&fixture.world, &source, 10.0));

    assert!(bundle.is_removed());
    assert!(
        bundle
            .get_item()
            .get(BUNDLE_CONTENTS)
            .expect("stack should keep bundle contents")
            .is_empty()
    );
    let dropped = drops(&fixture.world);
    assert_eq!(dropped.len(), 3);
    for entity in dropped {
        let item = entity
            .as_ref()
            .downcast_ref::<ItemEntity>()
            .expect("spawned entity should be an item");
        assert_eq!(item.position(), bundle.position());
        assert_eq!(item.get_thrower(), None);
        assert_eq!(item.get_pickup_delay(), 0);
    }
}

#[test]
#[should_panic(expected = "Bundle item use failed")]
fn bundle_review_invalid_runtime_contents_fail_the_use_tick() {
    use vecta_registry::data_components::vanilla_components::MAX_DAMAGE;

    let (fixture, player) = fixture("bundle_review_invalid_use_tick");
    let entity: SharedEntity = player;
    let mut invalid = ItemStack::new(&vanilla_items::STONE);
    invalid.set(MAX_DAMAGE, 5);
    let invalid =
        ItemStackTemplate::from_stack(&invalid).expect("template codec should accept the patch");
    let mut stack = ItemStack::new(&vanilla_items::BUNDLE);
    stack.set(
        BUNDLE_CONTENTS,
        BundleContents::new(vec![invalid.clone(), invalid]),
    );
    BundleItem.on_use_tick(
        &fixture.world,
        LivingEntityRef::new(&entity).expect("test player should be living"),
        &mut stack,
        200,
    );
}
