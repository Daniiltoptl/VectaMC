//! Advancement reward application.
use vecta_registry::loot_table::loot_context_params;
use vecta_registry::vanilla_context_key_sets as loot_context_param_sets;

use crate::entity::{Entity, entity_loot_ref, living_loot_equipment};
use crate::player::Player;
use crate::world::random::with_loot_source;
use vecta_registry::advancement::AdvancementRewards;
use vecta_registry::loot_table::LootContext;

/// Grants experience and loot rewards but recipes and functions are not handled yet
///
/// # Panics
///
/// Panics if the internally assembled loot parameters violate the reward contract.
pub fn grant_reward(player: &Player, reward: &AdvancementRewards) {
    player.give_experience_points(reward.experience);

    let position = player.position();
    let equipment = living_loot_equipment(player);
    let level = player.level();

    let mut changes = false;
    // Vanilla `AdvancementRewards.grant` builds one context per table.
    for loot_table in &reward.loots {
        let items = with_loot_source(level.as_deref(), loot_table, 0, |rng| {
            let mut ctx = LootContext::new(rng)
                .with_parameter(
                    loot_context_params::THIS_ENTITY,
                    entity_loot_ref(player, Some(&equipment)),
                )
                .with_parameter(
                    loot_context_params::ORIGIN,
                    (position.x, position.y, position.z),
                );
            let validation = ctx
                .params
                .validate(&loot_context_param_sets::ADVANCEMENT_REWARD);
            assert!(
                validation.is_ok(),
                "Invalid advancement loot parameters: {validation:?}"
            );
            loot_table.get_random_items(&mut ctx)
        });
        for mut item in items {
            if player.add_item_with_sound(&mut item) {
                changes = true;
                continue;
            }
            if let Some(drop) = player.spawn_dropped_item(item, false, false) {
                drop.set_no_pickup_delay();
                drop.set_owner(Some(player.gameprofile.id()));
            }
        }
    }

    if changes {
        player.broadcast_inventory_changes();
    }

    //TODO grant recipes and functions when implemented
}
