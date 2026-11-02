//! Dispenser shears (W06-061…065).

use super::*;
use crate::entity::entities::{ItemEntity, SheepEntity};
use vecta_registry::level_events::{
    PARTICLES_SHOOT_SMOKE, SOUND_DISPENSER_DISPENSE, SOUND_DISPENSER_FAIL,
};

/// The block in front of [`TEST_POS`] for a south-facing dispenser.
const TARGET: BlockPos = BlockPos::new(4, 64, 5);

fn spawn_sheep(fixture: &TestWorld, position: DVec3) -> Arc<SheepEntity> {
    let sheep = Arc::new(SheepEntity::new(
        &vanilla_entities::SHEEP,
        next_entity_id(),
        position,
        Arc::downgrade(&fixture.world),
    ));
    fixture
        .world
        .try_add_entity(Arc::clone(&sheep) as SharedEntity)
        .expect("sheep in a loaded chunk");
    sheep
}

fn dispense_shears(name: &'static str) -> (TestWorld, BlockPos, SentPackets) {
    let (fixture, pos, _player, packets) = listening_dispenser_fixture(name, Direction::South);
    set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::SHEARS));
    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );
    (fixture, pos, packets)
}

fn wool_drops(fixture: &TestWorld, position: DVec3) -> Vec<ItemStack> {
    let mut drops = Vec::new();
    for entity in dropped_items(&fixture.world, position) {
        if let Some(item) = entity.downcast_ref::<ItemEntity>() {
            let stack = item.get_item();
            drops.push(stack.copy_with_count(stack.count()));
        }
    }
    drops
}

#[test]
fn a_dispensed_shears_shears_the_sheep_in_front() {
    let (fixture, pos, _player, packets) =
        listening_dispenser_fixture("dispenser_shears_sheep", Direction::South);
    let sheep = spawn_sheep(&fixture, DVec3::new(4.5, 64.0, 5.5));
    set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::SHEARS));

    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );

    assert!(
        sheep.is_sheared(),
        "vanilla `Shearable.shear` marks the sheep as sheared"
    );
    let drops = wool_drops(&fixture, sheep.position());
    assert!(
        !drops.is_empty(),
        "the shearing loot table has to produce the wool drops"
    );
    assert!(
        drops.iter().all(|drop| drop.is(&vanilla_items::WHITE_WOOL)),
        "a default sheep drops white wool, got {:?}",
        drops
            .iter()
            .map(|drop| drop.item().key.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        item_at(&fixture, pos, 0).get_damage_value(),
        1,
        "a successful shear wears the shears once"
    );
    assert_eq!(
        sent_level_events(&packets),
        [
            (SOUND_DISPENSER_DISPENSE, pos, 0),
            (
                PARTICLES_SHOOT_SMOKE,
                pos,
                Direction::South.get_3d_data_value()
            ),
        ]
    );
}

#[test]
fn a_dispensed_shears_leaves_an_already_sheared_sheep_alone() {
    let (fixture, pos, _player, packets) =
        listening_dispenser_fixture("dispenser_shears_sheared", Direction::South);
    let sheep = spawn_sheep(&fixture, DVec3::new(4.5, 64.0, 5.5));
    sheep.set_sheared(true);
    set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::SHEARS));

    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );

    assert!(
        wool_drops(&fixture, sheep.position())
            .iter()
            .all(|drop| !drop.is(&vanilla_items::WHITE_WOOL)),
        "vanilla `readyForShearing` refuses a sheep that already has no wool"
    );
    assert_eq!(
        item_at(&fixture, pos, 0).get_damage_value(),
        0,
        "a failed attempt must not wear the shears"
    );
    assert!(
        ejected_items(&fixture, pos).iter().any(|entity| entity
            .downcast_ref::<ItemEntity>()
            .is_some_and(|item| item.get_item().is(&vanilla_items::SHEARS))),
        "the unused shears are thrown out"
    );
    assert_eq!(
        sent_level_events(&packets)[0],
        (SOUND_DISPENSER_FAIL, pos, 0)
    );
}

#[test]
fn a_dispensed_shears_shears_only_one_of_two_sheep() {
    let (fixture, pos, _player, _packets) =
        listening_dispenser_fixture("dispenser_shears_two", Direction::South);
    let first = spawn_sheep(&fixture, DVec3::new(4.3, 64.0, 5.5));
    let second = spawn_sheep(&fixture, DVec3::new(4.7, 64.0, 5.5));
    set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::SHEARS));

    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );

    let sheared = [first.is_sheared(), second.is_sheared()]
        .into_iter()
        .filter(|sheared| *sheared)
        .count();
    assert_eq!(
        sheared, 1,
        "vanilla `tryShearEntity` returns after the first ready `Shearable`"
    );
    assert_eq!(
        item_at(&fixture, pos, 0).get_damage_value(),
        1,
        "one shear, one durability point"
    );
}

#[test]
fn a_dispensed_shears_over_nothing_is_thrown_out() {
    let (fixture, pos, packets) = dispense_shears("dispenser_shears_nothing");

    assert!(item_at(&fixture, pos, 0).is_empty());
    assert_eq!(ejected_items(&fixture, pos).len(), 1);
    assert_eq!(
        sent_level_events(&packets)[0],
        (SOUND_DISPENSER_FAIL, pos, 0)
    );
    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::AIR.default_state(),
        "a failed shear leaves the block in front unchanged"
    );
}
