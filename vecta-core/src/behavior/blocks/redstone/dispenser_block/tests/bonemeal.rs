//! Dispenser bone meal (W06-051…055).

use super::*;
use vecta_registry::blocks::properties::BlockStateProperties;
use vecta_registry::level_events::{
    PARTICLES_AND_SOUND_PLANT_GROWTH, PARTICLES_SHOOT_SMOKE, SOUND_DISPENSER_DISPENSE,
    SOUND_DISPENSER_FAIL,
};

/// The block in front of [`TEST_POS`] for a south-facing dispenser.
const TARGET: BlockPos = BlockPos::new(4, 64, 5);

fn pulse_bone_meal(
    name: &'static str,
    setup: impl FnOnce(&TestWorld),
) -> (TestWorld, BlockPos, SentPackets) {
    let (fixture, pos, _player, packets) = listening_dispenser_fixture(name, Direction::South);
    setup(&fixture);
    set_item_at(
        &fixture,
        pos,
        0,
        ItemStack::with_count(&vanilla_items::BONE_MEAL, 3),
    );
    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );
    (fixture, pos, packets)
}

#[test]
fn a_dispensed_bone_meal_grows_a_crop() {
    let (fixture, pos, packets) = pulse_bone_meal("dispenser_bonemeal_crop", |fixture| {
        assert!(fixture.world.set_block(
            TARGET.below(),
            vanilla_blocks::FARMLAND.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        assert!(fixture.world.set_block(
            TARGET,
            vanilla_blocks::WHEAT.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
    });

    let age = fixture
        .world
        .get_block_state(TARGET)
        .get_value(&BlockStateProperties::AGE_7);
    assert!(
        age > 0,
        "vanilla `CropBlock.performBonemeal` raises the age by 2..=5, got {age}"
    );
    assert_eq!(
        item_at(&fixture, pos, 0).count(),
        2,
        "a successful growth consumes exactly one bone meal"
    );
    assert_eq!(
        sent_level_events(&packets),
        [
            (PARTICLES_AND_SOUND_PLANT_GROWTH, TARGET, 15),
            (SOUND_DISPENSER_DISPENSE, pos, 0),
            (
                PARTICLES_SHOOT_SMOKE,
                pos,
                Direction::South.get_3d_data_value()
            ),
        ],
        "vanilla fires 1505 from `execute` and then the dispenser sound pair"
    );
}

#[test]
fn a_dispensed_bone_meal_refuses_a_max_age_crop() {
    let (fixture, pos, packets) = pulse_bone_meal("dispenser_bonemeal_max_age", |fixture| {
        assert!(fixture.world.set_block(
            TARGET.below(),
            vanilla_blocks::FARMLAND.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        let ripe = vanilla_blocks::WHEAT
            .default_state()
            .set_value(&BlockStateProperties::AGE_7, 7);
        assert!(
            fixture
                .world
                .set_block(TARGET, ripe, UpdateFlags::UPDATE_ALL)
        );
    });

    assert_eq!(
        fixture
            .world
            .get_block_state(TARGET)
            .get_value(&BlockStateProperties::AGE_7),
        7,
        "a ripe crop is not a valid bonemeal target"
    );
    assert_eq!(
        item_at(&fixture, pos, 0).count(),
        3,
        "vanilla `growCrop` returns false without shrinking the stack"
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
        ],
        "a failed growth plays 1001 and never 1505"
    );
}

#[test]
fn a_dispensed_bone_meal_grows_seagrass_in_water() {
    let (fixture, pos, packets) = pulse_bone_meal("dispenser_bonemeal_seagrass", |fixture| {
        assert!(fixture.world.set_block(
            TARGET.below(),
            vanilla_blocks::STONE.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        assert!(fixture.world.set_block(
            TARGET,
            vanilla_blocks::WATER.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
    });

    let grown = fixture.world.get_block_state(TARGET);
    assert!(
        grown.get_block() == &vanilla_blocks::SEAGRASS
            || grown.get_block() == &vanilla_blocks::TALL_SEAGRASS,
        "vanilla `growWaterPlant` places seagrass at the water source first, got {:?}",
        grown.get_block().key
    );
    assert_eq!(
        item_at(&fixture, pos, 0).count(),
        2,
        "the water-plant branch consumes one bone meal"
    );
    assert_eq!(
        sent_level_events(&packets)[0],
        (PARTICLES_AND_SOUND_PLANT_GROWTH, TARGET, 15)
    );
}

#[test]
fn a_dispensed_bone_meal_on_an_invalid_block_changes_nothing() {
    let (fixture, pos, packets) = pulse_bone_meal("dispenser_bonemeal_invalid", |fixture| {
        assert!(fixture.world.set_block(
            TARGET,
            vanilla_blocks::STONE.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
    });

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::STONE.default_state()
    );
    assert_eq!(
        item_at(&fixture, pos, 0).count(),
        3,
        "a failed growth must not spend bone meal"
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
