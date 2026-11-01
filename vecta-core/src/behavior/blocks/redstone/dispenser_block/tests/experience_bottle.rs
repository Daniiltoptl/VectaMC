use super::*;
use text_components::TextComponent;
use vecta_registry::data_components::vanilla_components::CUSTOM_NAME;

use crate::entity::AddEntityError;
use crate::entity::entities::ThrownExperienceBottleEntity;

/// Vanilla `ExperienceBottleItem.createDispenseConfig` launches at
/// `DEFAULT.power * 1.25` with `DEFAULT.uncertainty * 0.5`.
const VANILLA_EXPERIENCE_BOTTLE_POWER: f64 = 1.375;

#[test]
fn dispensed_experience_bottle_launches_one_projectile_from_the_java_position() {
    let (fixture, pos, _player, packets) =
        listening_dispenser_fixture("dispenser_experience_bottle", Direction::East);
    set_item_at(
        &fixture,
        pos,
        0,
        ItemStack::with_count(&vanilla_items::EXPERIENCE_BOTTLE, 2),
    );
    let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);

    pulse(&behavior, &fixture, pos);

    assert_eq!(
        item_at(&fixture, pos, 0).count(),
        1,
        "a dispensed bottle consumes exactly one item"
    );
    let projectiles = projectiles_of_type(&fixture, pos, &vanilla_entities::EXPERIENCE_BOTTLE);
    assert_eq!(
        projectiles.len(),
        1,
        "vanilla `ProjectileDispenseBehavior` spawns one projectile"
    );
    let Some(bottle) = projectiles[0].downcast_ref::<ThrownExperienceBottleEntity>() else {
        panic!("the dispensed entity is the experience bottle projectile");
    };
    assert!(
        bottle.get_item().is(&vanilla_items::EXPERIENCE_BOTTLE),
        "the projectile carries the item it was dispensed from"
    );
    assert_eq!(
        bottle.get_item().count(),
        1,
        "vanilla keeps a single-item copy on the projectile"
    );
    assert!(
        bottle.get_owner().is_none(),
        "a dispenser is not an entity, so vanilla gives the projectile no owner"
    );

    // Vanilla `DispenserBlock.getDispensePosition(source, 0.7, Vec3(0, 0.1, 0))`.
    let expected = DVec3::new(4.5 + 0.7, 64.5 + 0.1, 4.5);
    assert!(
        (projectiles[0].position() - expected).length() < 1e-9,
        "launched from {expected:?}, got {:?}",
        projectiles[0].position()
    );

    // The bottle's own config: power 1.375 and half the default spread. A
    // projectile registered with `DispenseConfig::DEFAULT` would launch at 1.1.
    let velocity = projectiles[0].velocity();
    assert!(
        (velocity.x - VANILLA_EXPERIENCE_BOTTLE_POWER).abs() <= 0.12
            && velocity.y.abs() <= 0.12
            && velocity.z.abs() <= 0.12,
        "an east-facing dispenser launches at power 1.375, got {velocity:?}"
    );

    assert_eq!(
        sent_level_events(&packets),
        [
            (SOUND_DISPENSER_PROJECTILE_LAUNCH, pos, 0),
            (
                PARTICLES_SHOOT_SMOKE,
                pos,
                Direction::East.get_3d_data_value()
            ),
        ],
        "the bottle uses the inherited launch sound and smoke animation"
    );
}

#[test]
fn dispensed_experience_bottle_keeps_the_dispensed_stack_components() {
    let (fixture, pos, _player, _packets) =
        listening_dispenser_fixture("dispenser_experience_bottle_components", Direction::Up);
    let mut stack = ItemStack::with_count(&vanilla_items::EXPERIENCE_BOTTLE, 3);
    stack.set(CUSTOM_NAME, TextComponent::plain("Bottled experience"));
    let expected_projection = stack.copy_with_count(1);
    set_item_at(&fixture, pos, 0, stack);
    let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);

    pulse(&behavior, &fixture, pos);

    assert_eq!(
        item_at(&fixture, pos, 0).count(),
        2,
        "the source stack loses exactly one item"
    );
    let projectiles = projectiles_of_type(&fixture, pos, &vanilla_entities::EXPERIENCE_BOTTLE);
    let Some(bottle) = projectiles[0].downcast_ref::<ThrownExperienceBottleEntity>() else {
        panic!("the dispensed entity is the experience bottle projectile");
    };
    assert_eq!(
        bottle.get_item(),
        expected_projection,
        "the projectile keeps the dispensed stack's components at count one"
    );
}

#[test]
fn rejected_experience_bottle_insertion_still_consumes_and_emits_events() {
    let (fixture, _, _player, packets) =
        listening_dispenser_fixture("dispenser_experience_bottle_rejection", Direction::East);
    let pos = BlockPos::new(15, 64, 4);
    assert!(
        fixture.world.set_block(
            pos,
            vanilla_blocks::DISPENSER
                .default_state()
                .set_value(FACING, Direction::East),
            UpdateFlags::UPDATE_ALL,
        )
    );
    // The source chunk is loaded; the spawn position lies in the unloaded next chunk.
    let rejected = Arc::new(ThrownExperienceBottleEntity::new(
        &vanilla_entities::EXPERIENCE_BOTTLE,
        next_entity_id(),
        DVec3::new(16.2, 64.6, 4.5),
        Arc::downgrade(&fixture.world),
    ));
    assert!(matches!(
        fixture.world.try_add_entity(rejected),
        Err(AddEntityError::ChunkNotLoaded { .. })
    ));
    set_item_at(
        &fixture,
        pos,
        0,
        ItemStack::with_count(&vanilla_items::EXPERIENCE_BOTTLE, 2),
    );
    packets.lock().clear();

    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );

    assert_eq!(item_at(&fixture, pos, 0).count(), 1);
    assert!(
        projectiles_of_type(&fixture, pos, &vanilla_entities::EXPERIENCE_BOTTLE).is_empty(),
        "the rejected projectile never enters the world"
    );
    assert_eq!(
        sent_level_events(&packets),
        [
            (SOUND_DISPENSER_PROJECTILE_LAUNCH, pos, 0),
            (
                PARTICLES_SHOOT_SMOKE,
                pos,
                Direction::East.get_3d_data_value()
            ),
        ],
        "vanilla ignores the insertion result, so consumption and events still happen"
    );
}
