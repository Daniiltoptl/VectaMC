use std::sync::Arc;

use glam::DVec3;
use steel_registry::item_stack::ItemStack;
use steel_registry::stat::{Stat, vanilla_stat_types};
use steel_registry::{init_vanilla_registry, vanilla_entities, vanilla_items};
use steel_utils::types::{GameType, InteractionHand};
use steel_utils::{ChunkPos, WorldAabb};

use crate::behavior::{ITEM_BEHAVIORS, InteractionResult, UseItemContext, init_behaviors};
use crate::entity::entities::ThrownExperienceBottleEntity;
use crate::entity::{Entity, ThrowableItemProjectile};
use crate::test_support::{TestPlayerBuilder, fresh_test_world, insert_ready_full_chunk};
use steel_utils::Downcast as _;

#[test]
fn experience_bottle_use_spawns_owned_projectile_and_consumes_only_in_survival() {
    init_vanilla_registry();
    init_behaviors();
    for game_mode in [GameType::Survival, GameType::Creative] {
        for hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
            for count in [1, 4] {
                let fixture = fresh_test_world("experience_bottle_use");
                let world = &fixture.world;
                insert_ready_full_chunk(world, ChunkPos::new(0, 0));
                let position = DVec3::new(0.5, 80.0, 0.5);
                let player = TestPlayerBuilder::new(Arc::clone(world), "BottleThrower", 1).build();
                player.try_set_position(position).expect("position player");
                player.set_rotation((0.0, 0.0));
                player.restore_game_modes(game_mode, None);
                player.abilities.lock().update_for_game_mode(game_mode);
                player.inventory.lock().set_item_in_hand(
                    hand,
                    ItemStack::with_count(&vanilla_items::EXPERIENCE_BOTTLE, count),
                );
                let mut context =
                    UseItemContext::new(&player, hand, world, Arc::clone(&player.inventory));
                let behavior = ITEM_BEHAVIORS.get_behavior(&vanilla_items::EXPERIENCE_BOTTLE);
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
                let bottles: Vec<_> = entities
                    .iter()
                    .filter(|entity| entity.entity_type() == &vanilla_entities::EXPERIENCE_BOTTLE)
                    .collect();
                assert_eq!(bottles.len(), 1);
                let bottle = bottles[0]
                    .downcast_ref::<ThrownExperienceBottleEntity>()
                    .expect("registered concrete bottle projectile");
                assert_eq!(bottle.projectile_owner_uuid(), Some(player.uuid()));
                assert_eq!(bottle.get_item().count(), 1);
                assert_eq!(bottle.get_item().item(), &*vanilla_items::EXPERIENCE_BOTTLE);
                assert!(bottle.velocity().y > 0.0);
                assert!(bottle.velocity().z > 0.5);
                let used = Stat::new(
                    &vanilla_stat_types::ITEM_USED,
                    &vanilla_items::EXPERIENCE_BOTTLE,
                );
                assert!(player.stats().contains(&(used, 1)));
            }
        }
    }
}
