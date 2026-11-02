//! Dispenser flint and steel (W06-046…050).
//!
//! Vanilla registers `Items.FIRE_CHARGE` as a projectile, so the fire-charge
//! half of G010 waits for a `SmallFireball` entity (G047).

use super::*;
use crate::entity::entities::PrimedTntEntity;
use vecta_registry::blocks::properties::BlockStateProperties;
use vecta_registry::level_events::{
    PARTICLES_SHOOT_SMOKE, SOUND_DISPENSER_DISPENSE, SOUND_DISPENSER_FAIL,
};
use vecta_registry::vanilla_game_rules;

/// The block in front of [`TEST_POS`] for a south-facing dispenser.
const TARGET: BlockPos = BlockPos::new(4, 64, 5);

fn flint_damage(fixture: &TestWorld, pos: BlockPos) -> i32 {
    item_at(fixture, pos, 0).get_damage_value()
}

#[test]
fn a_dispensed_flint_lights_an_unlit_campfire() {
    let (fixture, pos, packets) = {
        let (fixture, pos, _player, packets) =
            listening_dispenser_fixture("dispenser_flint_campfire", Direction::South);
        let unlit = vanilla_blocks::CAMPFIRE
            .default_state()
            .set_value(&BlockStateProperties::LIT, false)
            .set_value(&BlockStateProperties::WATERLOGGED, false);
        assert!(
            fixture
                .world
                .set_block(TARGET, unlit, UpdateFlags::UPDATE_ALL)
        );
        set_item_at(
            &fixture,
            pos,
            0,
            ItemStack::new(&vanilla_items::FLINT_AND_STEEL),
        );
        pulse(
            &DispenserBlock::new(&vanilla_blocks::DISPENSER),
            &fixture,
            pos,
        );
        (fixture, pos, packets)
    };

    let state = fixture.world.get_block_state(TARGET);
    assert_eq!(
        state.get_block(),
        &vanilla_blocks::CAMPFIRE,
        "lighting changes only the LIT property"
    );
    assert_eq!(
        state.try_get_value(&BlockStateProperties::LIT),
        Some(true),
        "vanilla `CampfireBlock.canLight` accepts an unlit, dry campfire"
    );
    assert_eq!(
        flint_damage(&fixture, pos),
        1,
        "a successful ignition wears the flint once"
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
fn a_dispensed_flint_leaves_an_already_lit_campfire_alone() {
    let (fixture, pos, _player, packets) =
        listening_dispenser_fixture("dispenser_flint_lit_campfire", Direction::South);
    let lit = vanilla_blocks::CAMPFIRE
        .default_state()
        .set_value(&BlockStateProperties::LIT, true)
        .set_value(&BlockStateProperties::WATERLOGGED, false);
    assert!(
        fixture
            .world
            .set_block(TARGET, lit, UpdateFlags::UPDATE_ALL)
    );
    set_item_at(
        &fixture,
        pos,
        0,
        ItemStack::new(&vanilla_items::FLINT_AND_STEEL),
    );

    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        lit,
        "vanilla `CampfireBlock.canLight` rejects a lit campfire"
    );
    // Vanilla keeps `success = true` when no branch runs, so the dispenser path
    // still wears the flint. This is the dispenser quirk, not the hand path.
    assert_eq!(flint_damage(&fixture, pos), 1);
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
fn a_dispensed_flint_lights_a_candle() {
    let (fixture, pos, _player, _packets) =
        listening_dispenser_fixture("dispenser_flint_candle", Direction::South);
    let unlit = vanilla_blocks::CANDLE
        .default_state()
        .set_value(&BlockStateProperties::LIT, false)
        .set_value(&BlockStateProperties::WATERLOGGED, false);
    assert!(
        fixture
            .world
            .set_block(TARGET, unlit, UpdateFlags::UPDATE_ALL)
    );
    set_item_at(
        &fixture,
        pos,
        0,
        ItemStack::new(&vanilla_items::FLINT_AND_STEEL),
    );

    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );

    assert_eq!(
        fixture
            .world
            .get_block_state(TARGET)
            .try_get_value(&BlockStateProperties::LIT),
        Some(true)
    );
}

#[test]
fn a_dispensed_flint_creates_fire_on_an_empty_target() {
    let (fixture, pos, _player, _packets) =
        listening_dispenser_fixture("dispenser_flint_fire", Direction::South);
    assert!(fixture.world.set_block(
        TARGET.below(),
        vanilla_blocks::STONE.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    set_item_at(
        &fixture,
        pos,
        0,
        ItemStack::new(&vanilla_items::FLINT_AND_STEEL),
    );

    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::FIRE.default_state(),
        "vanilla `BaseFireBlock.getState` picks the fire block for the target"
    );
    assert_eq!(flint_damage(&fixture, pos), 1);
}

#[test]
fn a_dispensed_flint_does_not_light_an_occupied_target() {
    let (fixture, pos, _player, _packets) =
        listening_dispenser_fixture("dispenser_flint_occupied", Direction::South);
    assert!(fixture.world.set_block(
        TARGET,
        vanilla_blocks::STONE.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    set_item_at(
        &fixture,
        pos,
        0,
        ItemStack::new(&vanilla_items::FLINT_AND_STEEL),
    );

    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::STONE.default_state(),
        "a full block is not air and cannot be lit, so the world is unchanged"
    );
}

#[test]
fn a_dispensed_flint_primes_tnt_and_removes_the_block() {
    let (fixture, pos, packets) = {
        let (fixture, pos, _player, packets) =
            listening_dispenser_fixture("dispenser_flint_tnt", Direction::South);
        assert!(fixture.world.set_block(
            TARGET,
            vanilla_blocks::TNT.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        set_item_at(
            &fixture,
            pos,
            0,
            ItemStack::new(&vanilla_items::FLINT_AND_STEEL),
        );
        pulse(
            &DispenserBlock::new(&vanilla_blocks::DISPENSER),
            &fixture,
            pos,
        );
        (fixture, pos, packets)
    };

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::AIR.default_state(),
        "vanilla removes the TNT block after priming it"
    );
    let primed: Vec<_> = fixture
        .world
        .get_entities_in_aabb(&WorldAabb::of_size(
            DVec3::new(4.5, 64.0, 5.5),
            2.0,
            2.0,
            2.0,
        ))
        .into_iter()
        .filter(|entity| entity.downcast_ref::<PrimedTntEntity>().is_some())
        .collect();
    assert_eq!(primed.len(), 1, "vanilla spawns one `PrimedTnt`");
    assert_eq!(flint_damage(&fixture, pos), 1);
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
fn tnt_explodes_off_turns_a_flint_failure_into_the_failure_sound() {
    let (fixture, pos, _player, packets) =
        listening_dispenser_fixture("dispenser_flint_tnt_disabled", Direction::South);
    assert!(
        fixture
            .world
            .set_game_rule(&vanilla_game_rules::TNT_EXPLODES, false)
    );
    assert!(
        !fixture
            .world
            .get_game_rule(&vanilla_game_rules::TNT_EXPLODES)
    );
    assert!(fixture.world.set_block(
        TARGET,
        vanilla_blocks::TNT.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    set_item_at(
        &fixture,
        pos,
        0,
        ItemStack::new(&vanilla_items::FLINT_AND_STEEL),
    );

    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::TNT.default_state(),
        "`TntBlock.prime` refuses to prime while `TNT_EXPLODES` is off"
    );
    assert_eq!(
        flint_damage(&fixture, pos),
        0,
        "vanilla only wears the flint when `isSuccess()` is still true"
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
        "vanilla `OptionalDispenseItemBehavior` plays 1001 instead of 1000 on failure"
    );
}

#[test]
fn a_fire_charge_stays_unregistered_until_the_small_fireball_exists() {
    use crate::behavior::blocks::redstone::dispenser_block::DISPENSE_BEHAVIORS;

    init_globals();
    assert!(
        DISPENSE_BEHAVIORS
            .get(&vanilla_items::FLINT_AND_STEEL)
            .is_some(),
        "the flint behavior is registered"
    );
    assert!(
        DISPENSE_BEHAVIORS
            .get(&vanilla_items::FIRE_CHARGE)
            .is_none(),
        "vanilla registers `Items.FIRE_CHARGE` as a projectile; registering direct ignition here would replace the `SmallFireball` with a block change"
    );
}
