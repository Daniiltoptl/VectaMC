//! Dispenser glass bottle (W06-056…060).

use super::*;
use crate::entity::entities::ItemEntity;
use vecta_registry::blocks::properties::BlockStateProperties;
use vecta_registry::data_components::vanilla_components::POTION_CONTENTS;
use vecta_registry::level_events::{
    PARTICLES_SHOOT_SMOKE, SOUND_DISPENSER_DISPENSE, SOUND_DISPENSER_FAIL,
};
use vecta_registry::vanilla_potions;

/// The block in front of [`TEST_POS`] for a south-facing dispenser.
const TARGET: BlockPos = BlockPos::new(4, 64, 5);

fn dispense_bottles(
    name: &'static str,
    setup: impl FnOnce(&TestWorld),
    count: i32,
) -> (TestWorld, BlockPos, SentPackets) {
    let (fixture, pos, _player, packets) = listening_dispenser_fixture(name, Direction::South);
    setup(&fixture);
    set_item_at(
        &fixture,
        pos,
        0,
        ItemStack::with_count(&vanilla_items::GLASS_BOTTLE, count),
    );
    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );
    (fixture, pos, packets)
}

fn water_at(fixture: &TestWorld, state: BlockStateId) {
    assert!(
        fixture
            .world
            .set_block(TARGET, state, UpdateFlags::UPDATE_ALL)
    );
}

#[test]
fn a_dispensed_bottle_fills_from_a_water_source() {
    let (fixture, pos, packets) = dispense_bottles(
        "dispenser_bottle_water",
        |fixture| water_at(fixture, vanilla_blocks::WATER.default_state()),
        1,
    );

    let filled = item_at(&fixture, pos, 0);
    assert!(
        filled.is(&vanilla_items::POTION),
        "vanilla replaces the bottle with `PotionContents.createItemStack(POTION, WATER)`"
    );
    assert!(
        filled
            .get(POTION_CONTENTS)
            .is_some_and(|contents| contents.is(&vanilla_potions::WATER)),
        "the potion has to carry the water contents, not an empty potion"
    );
    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::WATER.default_state(),
        "filling a bottle never consumes the water source"
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
fn a_dispensed_bottle_also_fills_from_flowing_water() {
    let (fixture, pos, _packets) = dispense_bottles(
        "dispenser_bottle_flowing",
        |fixture| {
            water_at(
                fixture,
                vanilla_blocks::WATER
                    .default_state()
                    .set_value(&BlockStateProperties::LEVEL, 7),
            );
        },
        1,
    );
    let flowing = fixture.world.get_block_state(TARGET);

    assert!(
        item_at(&fixture, pos, 0).is(&vanilla_items::POTION),
        "vanilla reads the fluid state tag, so flowing water fills a dispensed bottle even though the hand path only ray-traces sources"
    );
    assert_eq!(fixture.world.get_block_state(TARGET), flowing);
}

#[test]
fn a_dispensed_bottle_takes_honey_from_a_full_hive() {
    let (fixture, pos, _packets) = dispense_bottles(
        "dispenser_bottle_honey",
        |fixture| {
            let full = vanilla_blocks::BEEHIVE
                .default_state()
                .set_value(&BlockStateProperties::LEVEL_HONEY, 5);
            assert!(
                fixture
                    .world
                    .set_block(TARGET, full, UpdateFlags::UPDATE_ALL)
            );
        },
        1,
    );

    assert_eq!(
        item_at(&fixture, pos, 0),
        ItemStack::new(&vanilla_items::HONEY_BOTTLE),
        "a full hive gives a honey bottle"
    );
    assert_eq!(
        fixture
            .world
            .get_block_state(TARGET)
            .try_get_value(&BlockStateProperties::LEVEL_HONEY),
        Some(0),
        "vanilla `resetHoneyLevel` clears the hive"
    );
    assert_eq!(
        fixture.world.get_block_state(TARGET).get_block(),
        &vanilla_blocks::BEEHIVE
    );
}

#[test]
fn a_dispensed_bottle_refuses_a_partial_hive() {
    let (fixture, pos, packets) = dispense_bottles(
        "dispenser_bottle_partial_honey",
        |fixture| {
            let partial = vanilla_blocks::BEEHIVE
                .default_state()
                .set_value(&BlockStateProperties::LEVEL_HONEY, 3);
            assert!(
                fixture
                    .world
                    .set_block(TARGET, partial, UpdateFlags::UPDATE_ALL)
            );
        },
        2,
    );
    let partial = fixture.world.get_block_state(TARGET);

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        partial,
        "a hive below honey level 5 is untouched"
    );
    assert_eq!(
        item_at(&fixture, pos, 0).count(),
        1,
        "the failed attempt throws out one bottle"
    );
    assert_eq!(
        sent_level_events(&packets),
        [
            (SOUND_DISPENSER_FAIL, pos, 0),
            (
                PARTICLES_SHOOT_SMOKE,
                pos,
                Direction::South.get_3d_data_value()
            ),
        ]
    );
}

#[test]
fn a_dispensed_bottle_on_an_invalid_target_is_thrown_out() {
    let (fixture, pos, packets) = dispense_bottles(
        "dispenser_bottle_invalid",
        |fixture| {
            assert!(fixture.world.set_block(
                TARGET,
                vanilla_blocks::STONE.default_state(),
                UpdateFlags::UPDATE_ALL,
            ));
        },
        1,
    );

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::STONE.default_state()
    );
    assert!(item_at(&fixture, pos, 0).is_empty());
    let ejected = ejected_items(&fixture, pos);
    assert_eq!(ejected.len(), 1);
    assert!(
        ejected[0]
            .downcast_ref::<ItemEntity>()
            .is_some_and(|item| item.get_item().is(&vanilla_items::GLASS_BOTTLE)),
        "vanilla falls back to `DefaultDispenseItemBehavior`, which throws the bottle out"
    );
    assert_eq!(
        sent_level_events(&packets)[0],
        (SOUND_DISPENSER_FAIL, pos, 0)
    );
}

#[test]
fn a_filled_bottle_goes_into_a_free_slot() {
    let (fixture, pos, _packets) = dispense_bottles(
        "dispenser_bottle_slots",
        |fixture| water_at(fixture, vanilla_blocks::WATER.default_state()),
        2,
    );

    assert_eq!(
        item_at(&fixture, pos, 0).count(),
        1,
        "one bottle leaves the dispensed stack"
    );
    assert!(
        item_at(&fixture, pos, 1).is(&vanilla_items::POTION),
        "the filled bottle takes the first free slot"
    );
    assert!(ejected_items(&fixture, pos).is_empty());
}

#[test]
fn a_full_dispenser_ejects_the_filled_bottle() {
    let (fixture, pos, _packets) = {
        let (fixture, pos, _player, packets) =
            listening_dispenser_fixture("dispenser_bottle_full", Direction::South);
        water_at(&fixture, vanilla_blocks::WATER.default_state());
        for slot in 0..DISPENSER_SLOTS {
            set_item_at(
                &fixture,
                pos,
                slot,
                ItemStack::with_count(&vanilla_items::GLASS_BOTTLE, 16),
            );
        }
        pulse(
            &DispenserBlock::new(&vanilla_blocks::DISPENSER),
            &fixture,
            pos,
        );
        (fixture, pos, packets)
    };

    assert_eq!(
        (0..DISPENSER_SLOTS)
            .filter(|&slot| item_at(&fixture, pos, slot).is(&vanilla_items::POTION))
            .count(),
        0,
        "no slot had room for the filled bottle"
    );
    assert_eq!(
        total_items(&fixture, pos, DISPENSER_SLOTS),
        9 * 16 - 1,
        "the dispensed bottle is gone from its stack"
    );
    let ejected = ejected_items(&fixture, pos);
    assert_eq!(ejected.len(), 1);
    assert!(
        ejected[0]
            .downcast_ref::<ItemEntity>()
            .is_some_and(|item| item.get_item().is(&vanilla_items::POTION)),
        "vanilla `addToInventoryOrDispense` throws the filled bottle out"
    );
}
