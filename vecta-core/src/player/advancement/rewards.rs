//! use to manage rewards

use crate::entity::{Entity, entity_loot_ref, living_loot_equipment};
use crate::player::Player;
use crate::world::random::with_loot_source;
use vecta_registry::advancement::AdvancementRewards;
use vecta_registry::loot_table::LootContext;

/// Grants experience and loot rewards but recipes and functions are not handled yet
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
                .with_this_entity(entity_loot_ref(player, Some(&equipment)))
                .with_origin(position.x, position.y, position.z);
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
