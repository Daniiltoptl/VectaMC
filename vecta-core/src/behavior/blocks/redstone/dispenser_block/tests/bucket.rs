//! Filled bucket dispenser behavior (W06-026…030).

use super::*;
use crate::entity::entities::ItemEntity;
use crate::test_support::played_sounds;
use glam::IVec3;
use vecta_registry::blocks::properties::BlockStateProperties;
use vecta_registry::sound_events;

/// The block in front of [`TEST_POS`] for a south-facing dispenser.
const TARGET: BlockPos = BlockPos::new(4, 64, 5);

/// `TARGET`'s centre encoded the way `C_SOUND` carries positions: `pos * 8`.
const TARGET_SOUND_POS: IVec3 = IVec3::new(36, 516, 44);

/// Places a south-facing dispenser holding `stack` in slot 0 and pulses it.
fn pulse_south_dispenser(
    name: &'static str,
    stack: ItemStack,
) -> (TestWorld, BlockPos, SentPackets) {
    let (fixture, pos, _player, packets) = listening_dispenser_fixture(name, Direction::South);
    set_item_at(&fixture, pos, 0, stack);
    let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);
    pulse(&behavior, &fixture, pos);
    (fixture, pos, packets)
}

#[test]
fn a_dispensed_water_bucket_places_a_source_and_leaves_an_empty_bucket() {
    let (fixture, pos, packets) = pulse_south_dispenser(
        "dispenser_water_bucket",
        ItemStack::new(&vanilla_items::WATER_BUCKET),
    );

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::WATER.default_state(),
        "vanilla places `content.defaultFluidState().createLegacyBlock()`"
    );
    assert!(
        fixture
            .world
            .get_block_state(TARGET)
            .get_fluid_state()
            .is_source(),
        "a bucket always places a source fluid, never a flowing level"
    );
    assert_eq!(
        item_at(&fixture, pos, 0),
        ItemStack::new(&vanilla_items::BUCKET),
        "a bucket stacks to one, so the remainder takes over the emptied slot"
    );
    assert!(
        ejected_items(&fixture, pos).is_empty(),
        "vanilla returns the empty bucket into the inventory, not the world"
    );

    let sounds = played_sounds(&packets);
    assert_eq!(sounds.len(), 1, "vanilla `playEmptySound` plays once");
    assert_eq!(sounds[0].0, sound_events::ITEM_BUCKET_EMPTY.key);
    assert_eq!(
        sounds[0].2, TARGET_SOUND_POS,
        "the sound comes from the block that received the water"
    );
    assert!((sounds[0].3 - 1.0).abs() < f32::EPSILON);
}

#[test]
fn a_dispensed_water_bucket_waterlogs_a_liquid_container() {
    let (fixture, pos, _player, packets) =
        listening_dispenser_fixture("dispenser_water_waterlog", Direction::South);
    assert!(fixture.world.set_block(
        TARGET,
        vanilla_blocks::OAK_SLAB.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    set_item_at(
        &fixture,
        pos,
        0,
        ItemStack::new(&vanilla_items::WATER_BUCKET),
    );
    let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);
    pulse(&behavior, &fixture, pos);

    let state = fixture.world.get_block_state(TARGET);
    assert_eq!(
        state.get_block(),
        &vanilla_blocks::OAK_SLAB,
        "waterlogging keeps the clicked block"
    );
    assert_eq!(
        state.try_get_value(&BlockStateProperties::WATERLOGGED),
        Some(true),
        "vanilla places the source into the block instead of replacing it"
    );
    assert_eq!(
        item_at(&fixture, pos, 0),
        ItemStack::new(&vanilla_items::BUCKET)
    );
    assert_eq!(
        played_sounds(&packets)[0].0,
        sound_events::ITEM_BUCKET_EMPTY.key
    );
}

#[test]
fn a_dispensed_lava_bucket_places_lava_with_the_lava_sound() {
    let (fixture, pos, packets) = pulse_south_dispenser(
        "dispenser_lava_bucket",
        ItemStack::new(&vanilla_items::LAVA_BUCKET),
    );

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::LAVA.default_state(),
        "a lava bucket places lava, not water"
    );
    assert_eq!(
        item_at(&fixture, pos, 0),
        ItemStack::new(&vanilla_items::BUCKET)
    );

    assert_eq!(
        played_sounds(&packets)[0].0,
        sound_events::ITEM_BUCKET_EMPTY_LAVA.key,
        "vanilla picks the empty sound from the fluid tag, so lava must not reuse the water sound"
    );
}

#[test]
fn a_dispensed_powder_snow_bucket_places_the_block_and_keeps_the_remainder() {
    let (fixture, pos, packets) = pulse_south_dispenser(
        "dispenser_powder_snow_bucket",
        ItemStack::new(&vanilla_items::POWDER_SNOW_BUCKET),
    );

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::POWDER_SNOW.default_state(),
        "vanilla `SolidBucketItem.emptyContents` places the item's block"
    );
    assert_eq!(
        item_at(&fixture, pos, 0),
        ItemStack::new(&vanilla_items::BUCKET)
    );

    let sounds = played_sounds(&packets);
    assert_eq!(
        sounds[0].0,
        sound_events::ITEM_BUCKET_EMPTY_POWDER_SNOW.key,
        "a solid bucket plays its own `place_sound` component"
    );
    assert_eq!(sounds[0].2, TARGET_SOUND_POS);
}

#[test]
fn a_rejected_powder_snow_target_falls_back_to_ejecting_the_bucket() {
    let (fixture, pos, _player, _packets) =
        listening_dispenser_fixture("dispenser_powder_snow_rejected", Direction::South);
    assert!(fixture.world.set_block(
        TARGET,
        vanilla_blocks::STONE.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    set_item_at(
        &fixture,
        pos,
        0,
        ItemStack::new(&vanilla_items::POWDER_SNOW_BUCKET),
    );
    let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);
    pulse(&behavior, &fixture, pos);

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::STONE.default_state(),
        "`SolidBucketItem.emptyContents` refuses a non-empty target"
    );
    assert!(
        item_at(&fixture, pos, 0).is_empty(),
        "the failed placement falls back to `DefaultDispenseItemBehavior`, which takes the item"
    );
    let ejected = ejected_items(&fixture, pos);
    assert_eq!(ejected.len(), 1);
    let Some(item_entity) = ejected[0].downcast_ref::<ItemEntity>() else {
        panic!("a rejected bucket is thrown out as an item entity");
    };
    assert!(
        item_entity
            .get_item()
            .is(&vanilla_items::POWDER_SNOW_BUCKET),
        "vanilla throws out the bucket it could not empty"
    );
}

#[test]
fn a_full_dispenser_keeps_the_empty_bucket_in_the_emptied_slot() {
    let (fixture, pos, _player, _packets) =
        listening_dispenser_fixture("dispenser_water_bucket_full", Direction::South);
    // Every slot holds a bucket, so the random slot choice still dispenses a
    // water bucket and the dispenser has no free slot for the remainder.
    for slot in 0..DISPENSER_SLOTS {
        set_item_at(
            &fixture,
            pos,
            slot,
            ItemStack::new(&vanilla_items::WATER_BUCKET),
        );
    }

    let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);
    pulse(&behavior, &fixture, pos);

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::WATER.default_state()
    );
    let empty_slots: Vec<usize> = (0..DISPENSER_SLOTS)
        .filter(|&slot| item_at(&fixture, pos, slot).is(&vanilla_items::BUCKET))
        .collect();
    assert_eq!(
        empty_slots.len(),
        1,
        "the emptied slot receives the remainder even when every other slot is full"
    );
    assert_eq!(
        (0..DISPENSER_SLOTS)
            .filter(|&slot| item_at(&fixture, pos, slot).is(&vanilla_items::WATER_BUCKET))
            .count(),
        8,
        "the other eight buckets stay untouched"
    );
    assert!(
        ejected_items(&fixture, pos).is_empty(),
        "a completely full dispenser still must not drop the bucket it just emptied"
    );
}

#[test]
fn consume_with_remainder_stores_a_stackable_remainder_and_ejects_what_does_not_fit() {
    // Buckets stack to one, so `consumeWithRemainder` always returns the
    // remainder into the emptied slot and never reaches the inventory insert.
    // A stackable dispensed item is what exercises the other branch.
    let (fixture, pos) = dispenser_fixture("dispense_remainder", Direction::South);
    let state = fixture.world.get_block_state(pos);
    let source = BlockSource {
        world: &fixture.world,
        pos,
        state,
    };
    set_item_at(
        &fixture,
        pos,
        0,
        ItemStack::with_count(&vanilla_items::GLASS_BOTTLE, 2),
    );

    let remaining = DefaultDispenseItemBehavior::consume_with_remainder(
        &source,
        ItemStack::with_count(&vanilla_items::GLASS_BOTTLE, 2),
        ItemStack::new(&vanilla_items::HONEY_BOTTLE),
    );
    assert_eq!(
        remaining.count(),
        1,
        "vanilla shrinks the dispensed stack by exactly one"
    );
    assert!(
        item_at(&fixture, pos, 1).is(&vanilla_items::HONEY_BOTTLE),
        "the remainder goes into the dispenser's first free slot"
    );
    assert!(ejected_items(&fixture, pos).is_empty());

    for slot in 0..DISPENSER_SLOTS {
        set_item_at(
            &fixture,
            pos,
            slot,
            ItemStack::with_count(&vanilla_items::STONE, 64),
        );
    }
    let remaining = DefaultDispenseItemBehavior::consume_with_remainder(
        &source,
        ItemStack::with_count(&vanilla_items::GLASS_BOTTLE, 2),
        ItemStack::new(&vanilla_items::HONEY_BOTTLE),
    );
    assert_eq!(remaining.count(), 1);
    let ejected = ejected_items(&fixture, pos);
    assert_eq!(
        ejected.len(),
        1,
        "vanilla `addToInventoryOrDispense` throws out a remainder that does not fit"
    );
    let Some(item_entity) = ejected[0].downcast_ref::<ItemEntity>() else {
        panic!("the leftover remainder is an item entity");
    };
    assert!(item_entity.get_item().is(&vanilla_items::HONEY_BOTTLE));
}

#[test]
fn filled_bucket_feedback_includes_the_outer_wrapper_and_nested_failure_drop() {
    for (name, item, target, pairs) in [
        (
            "bucket_feedback_water",
            &*vanilla_items::WATER_BUCKET,
            vanilla_blocks::AIR.default_state(),
            1,
        ),
        (
            "bucket_feedback_lava",
            &*vanilla_items::LAVA_BUCKET,
            vanilla_blocks::AIR.default_state(),
            1,
        ),
        (
            "bucket_feedback_snow",
            &*vanilla_items::POWDER_SNOW_BUCKET,
            vanilla_blocks::AIR.default_state(),
            1,
        ),
        (
            "bucket_feedback_rejected",
            &*vanilla_items::POWDER_SNOW_BUCKET,
            vanilla_blocks::STONE.default_state(),
            2,
        ),
    ] {
        let (fixture, pos, _player, packets) = listening_dispenser_fixture(name, Direction::South);
        fixture
            .world
            .set_block(TARGET, target, UpdateFlags::UPDATE_ALL);
        set_item_at(&fixture, pos, 0, ItemStack::new(item));
        pulse(
            &DispenserBlock::new(&vanilla_blocks::DISPENSER),
            &fixture,
            pos,
        );
        let pair = [
            (SOUND_DISPENSER_DISPENSE, pos, 0),
            (
                PARTICLES_SHOOT_SMOKE,
                pos,
                Direction::South.get_3d_data_value(),
            ),
        ];
        assert_eq!(sent_level_events(&packets), pair.repeat(pairs));
    }
}

#[test]
fn a_lava_bucket_cannot_replace_a_waterloggable_slab() {
    let (fixture, pos, _player, _packets) =
        listening_dispenser_fixture("dispenser_lava_slab", Direction::South);
    let slab = vanilla_blocks::OAK_SLAB.default_state();
    assert!(
        fixture
            .world
            .set_block(TARGET, slab, UpdateFlags::UPDATE_ALL)
    );
    set_item_at(
        &fixture,
        pos,
        0,
        ItemStack::new(&vanilla_items::LAVA_BUCKET),
    );

    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );

    assert_eq!(fixture.world.get_block_state(TARGET), slab);
    assert!(item_at(&fixture, pos, 0).is_empty());
    let dropped = ejected_items(&fixture, pos);
    assert_eq!(dropped.len(), 1);
    assert!(
        dropped[0]
            .downcast_ref::<ItemEntity>()
            .expect("dropped bucket")
            .get_item()
            .is(&vanilla_items::LAVA_BUCKET)
    );
}
