use super::*;

use crate::entity::entities::ThrownEggEntity;

/// Vanilla `EggItem` does not override `createDispenseConfig`, so a dispensed egg
/// uses `DispenseConfig.DEFAULT` power 1.1 rather than the 1.5 of a hand throw.
const VANILLA_DISPENSED_EGG_POWER: f64 = 1.1;

#[test]
fn dispensed_eggs_launch_one_projectile_that_keeps_the_egg_item() {
    for (name, item) in [
        ("dispenser_egg", &*vanilla_items::EGG),
        ("dispenser_blue_egg", &*vanilla_items::BLUE_EGG),
        ("dispenser_brown_egg", &*vanilla_items::BROWN_EGG),
    ] {
        let (fixture, pos, _player, packets) = listening_dispenser_fixture(name, Direction::South);
        set_item_at(&fixture, pos, 0, ItemStack::with_count(item, 2));
        let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);

        pulse(&behavior, &fixture, pos);

        assert_eq!(
            item_at(&fixture, pos, 0).count(),
            1,
            "a dispensed egg consumes exactly one item"
        );
        let projectiles = projectiles_of_type(&fixture, pos, &vanilla_entities::EGG);
        assert_eq!(
            projectiles.len(),
            1,
            "all three egg colours share the single vanilla `minecraft:egg` entity type"
        );
        let Some(egg) = projectiles[0].downcast_ref::<ThrownEggEntity>() else {
            panic!("the dispensed entity is the egg projectile");
        };
        assert_eq!(
            egg.get_item(),
            ItemStack::new(item),
            "the projectile keeps the dispensed egg item, including its chicken variant"
        );

        let velocity = projectiles[0].velocity();
        assert!(
            (velocity.z - VANILLA_DISPENSED_EGG_POWER).abs() <= 0.12
                && velocity.x.abs() <= 0.12
                && velocity.y.abs() <= 0.12,
            "a south-facing dispenser launches at the default power 1.1, got {velocity:?}"
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
}
