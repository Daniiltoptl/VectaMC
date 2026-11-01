use super::*;

use crate::entity::entities::SplashPotionEntity;
use vecta_registry::data_components::PotionContents;
use vecta_registry::data_components::vanilla_components::POTION_CONTENTS;
use vecta_registry::vanilla_potions;

/// `ThrowablePotionItem.createDispenseConfig`: `DEFAULT.power * 1.25`.
const VANILLA_THROWN_POTION_POWER: f64 = 1.375;

fn splash_potion_stack() -> ItemStack {
    let mut stack = ItemStack::new(&vanilla_items::SPLASH_POTION);
    stack.set(
        POTION_CONTENTS,
        PotionContents::of(&vanilla_potions::SWIFTNESS),
    );
    stack
}

#[test]
fn dispensed_splash_potion_launches_one_unowned_projectile() {
    let (fixture, pos, _player, packets) =
        listening_dispenser_fixture("dispenser_splash_potion", Direction::South);
    set_item_at(&fixture, pos, 0, splash_potion_stack());
    let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);

    pulse(&behavior, &fixture, pos);

    assert!(
        item_at(&fixture, pos, 0).is_empty(),
        "the only potion is consumed"
    );
    let projectiles = projectiles_of_type(&fixture, pos, &vanilla_entities::SPLASH_POTION);
    assert_eq!(
        projectiles.len(),
        1,
        "vanilla `ProjectileDispenseBehavior` spawns one projectile"
    );
    assert!(
        projectiles[0]
            .as_projectile()
            .is_some_and(|projectile| projectile.projectile_owner().is_none()),
        "a dispenser is not an entity, so the potion has no owner"
    );

    let velocity = projectiles[0].velocity();
    assert!(
        (velocity.z - VANILLA_THROWN_POTION_POWER).abs() <= 0.12
            && velocity.x.abs() <= 0.12
            && velocity.y.abs() <= 0.12,
        "`ThrowablePotionItem` dispenses at power 1.375, got {velocity:?}"
    );

    assert_eq!(
        sent_level_events(&packets),
        [
            (SOUND_DISPENSER_PROJECTILE_LAUNCH, pos, 0),
            (
                PARTICLES_SHOOT_SMOKE,
                pos,
                Direction::South.get_3d_data_value()
            ),
        ],
    );
}

#[test]
fn dispensed_splash_potion_keeps_its_potion_contents() {
    let (fixture, pos, _player, _packets) =
        listening_dispenser_fixture("dispenser_splash_potion_contents", Direction::Up);
    let stack = splash_potion_stack();
    let expected = stack.copy_with_count(1);
    set_item_at(&fixture, pos, 0, stack);
    let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);

    pulse(&behavior, &fixture, pos);

    let projectiles = projectiles_of_type(&fixture, pos, &vanilla_entities::SPLASH_POTION);
    let Some(potion) = projectiles[0].downcast_ref::<SplashPotionEntity>() else {
        panic!("the dispensed entity is the splash potion projectile");
    };
    assert_eq!(
        potion.get_item(),
        expected,
        "the flying potion keeps the base potion and every effect component"
    );
}
