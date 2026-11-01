use std::sync::Arc;

use glam::DVec3;
use vecta_registry::data_components::vanilla_components::CHICKEN_VARIANT;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::stat::{Stat, vanilla_stat_types};
use vecta_registry::{
    init_vanilla_registry, vanilla_chicken_variants, vanilla_entities, vanilla_items,
};
use vecta_utils::types::{GameType, InteractionHand};
use vecta_utils::{ChunkPos, Downcast as _, WorldAabb};

use crate::behavior::{ITEM_BEHAVIORS, InteractionResult, UseItemContext, init_behaviors};
use crate::entity::entities::ThrownEggEntity;
use crate::entity::{Entity as _, ThrowableItemProjectile as _};
use crate::test_support::{TestPlayerBuilder, fresh_test_world, insert_ready_full_chunk};

/// Vanilla `EggItem.PROJECTILE_SHOOT_POWER`.
const VANILLA_EGG_POWER: f64 = 1.5;

#[test]
fn egg_use_spawns_owned_projectile_and_consumes_only_in_survival() {
    init_vanilla_registry();
    init_behaviors();
    for game_mode in [GameType::Survival, GameType::Creative] {
        for hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
            for count in [1, 4] {
                let fixture = fresh_test_world("egg_use");
                let world = &fixture.world;
                insert_ready_full_chunk(world, ChunkPos::new(0, 0));
                let position = DVec3::new(0.5, 80.0, 0.5);
                let player = TestPlayerBuilder::new(Arc::clone(world), "EggThrower", 1).build();
                player.try_set_position(position).expect("position player");
                player.set_rotation((0.0, 0.0));
                player.restore_game_modes(game_mode, None);
                player.abilities.lock().update_for_game_mode(game_mode);
                player
                    .inventory
                    .lock()
                    .set_item_in_hand(hand, ItemStack::with_count(&vanilla_items::EGG, count));
                let mut context =
                    UseItemContext::new(&player, hand, world, Arc::clone(&player.inventory));
                let behavior = ITEM_BEHAVIORS.get_behavior(&vanilla_items::EGG);
                assert_eq!(behavior.use_item(&mut context), InteractionResult::Success);

                let inventory = player.inventory.lock();
                let remaining = inventory.get_item_in_hand(hand);
                let expected = if game_mode == GameType::Creative {
                    count
                } else {
                    count - 1
                };
                assert_eq!(remaining.count(), expected);
                assert_eq!(remaining.is_empty(), expected == 0);
                drop(inventory);

                let entities =
                    world.get_entities_in_aabb(&WorldAabb::of_size(position, 8.0, 8.0, 8.0));
                let eggs: Vec<_> = entities
                    .iter()
                    .filter(|entity| entity.entity_type() == &vanilla_entities::EGG)
                    .collect();
                assert_eq!(eggs.len(), 1);
                let egg = eggs[0]
                    .downcast_ref::<ThrownEggEntity>()
                    .expect("registered concrete egg projectile");
                assert_eq!(egg.projectile_owner_uuid(), Some(player.uuid()));
                assert_eq!(egg.get_item().count(), 1);
                assert_eq!(egg.get_item().item(), &*vanilla_items::EGG);
                assert!(
                    (egg.velocity().z - VANILLA_EGG_POWER).abs() <= 0.2,
                    "vanilla throws an egg at power 1.5, got {:?}",
                    egg.velocity()
                );
                let used = Stat::new(&vanilla_stat_types::ITEM_USED, &vanilla_items::EGG);
                assert!(player.stats().contains(&(used, 1)));
            }
        }
    }
}

/// The blue and brown eggs differ only by the `chicken/variant` component on the
/// item, so the projectile has to carry that component for the hatched chick to
/// keep the colour.
#[test]
fn thrown_coloured_egg_keeps_its_variant_component() {
    init_vanilla_registry();
    init_behaviors();
    for (item, variant) in [
        (&vanilla_items::BLUE_EGG, &vanilla_chicken_variants::COLD),
        (&vanilla_items::BROWN_EGG, &vanilla_chicken_variants::WARM),
    ] {
        let fixture = fresh_test_world("egg_variant_throw");
        let world = &fixture.world;
        insert_ready_full_chunk(world, ChunkPos::new(0, 0));
        let position = DVec3::new(0.5, 80.0, 0.5);
        let player = TestPlayerBuilder::new(Arc::clone(world), "EggThrower", 1).build();
        player.try_set_position(position).expect("position player");
        player.set_rotation((0.0, 0.0));
        let stack = ItemStack::new(item);
        assert_eq!(
            stack
                .get(CHICKEN_VARIANT)
                .map(|value| value.value().key.clone()),
            Some(variant.key.clone()),
            "the extracted item data carries the variant"
        );
        player
            .inventory
            .lock()
            .set_item_in_hand(InteractionHand::MainHand, stack);
        let mut context = UseItemContext::new(
            &player,
            InteractionHand::MainHand,
            world,
            Arc::clone(&player.inventory),
        );
        assert_eq!(
            ITEM_BEHAVIORS.get_behavior(item).use_item(&mut context),
            InteractionResult::Success
        );

        let entities = world.get_entities_in_aabb(&WorldAabb::of_size(position, 8.0, 8.0, 8.0));
        let egg = entities
            .iter()
            .find(|entity| entity.entity_type() == &vanilla_entities::EGG)
            .and_then(|entity| entity.downcast_ref::<ThrownEggEntity>())
            .expect("the thrown egg is in the world");
        assert_eq!(egg.get_item().item(), &**item);
        assert_eq!(
            egg.get_item()
                .get(CHICKEN_VARIANT)
                .map(|value| value.value().key.clone()),
            Some(variant.key.clone()),
            "the projectile keeps the egg's variant component"
        );
    }
}
