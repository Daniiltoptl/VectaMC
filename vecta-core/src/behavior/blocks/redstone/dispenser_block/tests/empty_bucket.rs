//! Empty bucket dispenser behavior (W06-031…035).

use super::*;
use crate::entity::entities::ItemEntity;
use vecta_registry::blocks::properties::BlockStateProperties;

/// The block in front of [`TEST_POS`] for a south-facing dispenser.
const TARGET: BlockPos = BlockPos::new(4, 64, 5);

fn pulse_empty_bucket(name: &'static str) -> (TestWorld, BlockPos) {
    let (fixture, pos, _player, _packets) = listening_dispenser_fixture(name, Direction::South);
    set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::BUCKET));
    let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);
    pulse(&behavior, &fixture, pos);
    (fixture, pos)
}

#[test]
fn a_dispensed_empty_bucket_picks_up_a_water_source() {
    let (fixture, pos, _player, _packets) =
        listening_dispenser_fixture("dispenser_bucket_water", Direction::South);
    assert!(fixture.world.set_block(
        TARGET,
        vanilla_blocks::WATER.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::BUCKET));

    let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);
    pulse(&behavior, &fixture, pos);

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::AIR.default_state(),
        "vanilla `LiquidBlock.pickupBlock` replaces exactly the source it drained"
    );
    assert_eq!(
        item_at(&fixture, pos, 0),
        ItemStack::new(&vanilla_items::WATER_BUCKET),
        "the filled bucket takes over the emptied slot"
    );
    assert!(
        ejected_items(&fixture, pos).is_empty(),
        "a successful pickup never throws the bucket out"
    );
}

#[test]
fn flowing_water_does_not_fill_a_dispensed_bucket() {
    let (fixture, pos, _player, _packets) =
        listening_dispenser_fixture("dispenser_bucket_flowing", Direction::South);
    let flowing = vanilla_blocks::WATER
        .default_state()
        .set_value(&BlockStateProperties::LEVEL, 7);
    assert!(
        fixture
            .world
            .set_block(TARGET, flowing, UpdateFlags::UPDATE_ALL)
    );
    set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::BUCKET));

    let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);
    pulse(&behavior, &fixture, pos);

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        flowing,
        "vanilla only picks up `LEVEL == 0`; flowing fluid stays"
    );
    assert!(
        item_at(&fixture, pos, 0).is_empty(),
        "a rejected pickup falls back to `DefaultDispenseItemBehavior`"
    );
    let ejected = ejected_items(&fixture, pos);
    assert_eq!(ejected.len(), 1);
    let Some(item_entity) = ejected[0].downcast_ref::<ItemEntity>() else {
        panic!("the unused bucket is thrown out as an item entity");
    };
    assert!(item_entity.get_item().is(&vanilla_items::BUCKET));
}

#[test]
fn a_dispensed_empty_bucket_picks_up_lava() {
    let (fixture, pos, _player, _packets) =
        listening_dispenser_fixture("dispenser_bucket_lava", Direction::South);
    assert!(fixture.world.set_block(
        TARGET,
        vanilla_blocks::LAVA.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::BUCKET));

    let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);
    pulse(&behavior, &fixture, pos);

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::AIR.default_state()
    );
    assert_eq!(
        item_at(&fixture, pos, 0),
        ItemStack::new(&vanilla_items::LAVA_BUCKET),
        "the block decides the item through its own fluid, not a shared water rule"
    );
}

#[test]
fn a_dispensed_empty_bucket_picks_up_powder_snow() {
    let (fixture, pos, _player, _packets) =
        listening_dispenser_fixture("dispenser_bucket_powder_snow", Direction::South);
    assert!(fixture.world.set_block(
        TARGET,
        vanilla_blocks::POWDER_SNOW.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::BUCKET));

    let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);
    pulse(&behavior, &fixture, pos);

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::AIR.default_state()
    );
    assert_eq!(
        item_at(&fixture, pos, 0),
        ItemStack::new(&vanilla_items::POWDER_SNOW_BUCKET),
        "`PowderSnowBlock` is a `BucketPickup` that hands back its own bucket item"
    );
}

#[test]
fn a_dispensed_empty_bucket_drains_a_waterlogged_block() {
    let (fixture, pos, _player, _packets) =
        listening_dispenser_fixture("dispenser_bucket_waterlogged", Direction::South);
    let slab = vanilla_blocks::OAK_SLAB
        .default_state()
        .set_value(&BlockStateProperties::WATERLOGGED, true);
    assert!(
        fixture
            .world
            .set_block(TARGET, slab, UpdateFlags::UPDATE_ALL)
    );
    set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::BUCKET));

    let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);
    pulse(&behavior, &fixture, pos);

    let state = fixture.world.get_block_state(TARGET);
    assert_eq!(state.get_block(), &vanilla_blocks::OAK_SLAB);
    assert_eq!(
        state.try_get_value(&BlockStateProperties::WATERLOGGED),
        Some(false),
        "vanilla `SimpleWaterloggedBlock.pickupBlock` drains the block in place"
    );
    assert_eq!(
        item_at(&fixture, pos, 0),
        ItemStack::new(&vanilla_items::WATER_BUCKET)
    );
}

#[test]
fn a_full_dispenser_fills_one_bucket_without_duplicating() {
    let (fixture, pos, _player, _packets) =
        listening_dispenser_fixture("dispenser_bucket_full", Direction::South);
    assert!(fixture.world.set_block(
        TARGET,
        vanilla_blocks::WATER.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    for slot in 0..DISPENSER_SLOTS {
        set_item_at(&fixture, pos, slot, ItemStack::new(&vanilla_items::BUCKET));
    }

    let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);
    pulse(&behavior, &fixture, pos);

    assert_eq!(
        (0..DISPENSER_SLOTS)
            .filter(|&slot| item_at(&fixture, pos, slot).is(&vanilla_items::WATER_BUCKET))
            .count(),
        1,
        "exactly one empty bucket becomes a water bucket"
    );
    assert_eq!(
        (0..DISPENSER_SLOTS)
            .filter(|&slot| item_at(&fixture, pos, slot).is(&vanilla_items::BUCKET))
            .count(),
        8,
        "the neighbouring stacks are untouched"
    );
    assert_eq!(
        total_items(&fixture, pos, DISPENSER_SLOTS),
        9,
        "nine buckets stay nine items, so the pickup neither duplicates nor drops one"
    );
}

#[test]
fn draining_a_source_updates_the_block_above_it() {
    let (fixture, pos, _player, _packets) =
        listening_dispenser_fixture("dispenser_bucket_neighbours", Direction::South);
    let above = TARGET.above();
    assert!(fixture.world.set_block(
        TARGET,
        vanilla_blocks::WATER.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    assert!(fixture.world.set_block(
        above,
        vanilla_blocks::LILY_PAD.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::BUCKET));

    let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);
    pulse(&behavior, &fixture, pos);

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::AIR.default_state()
    );
    assert_eq!(
        fixture.world.get_block_state(above),
        vanilla_blocks::AIR.default_state(),
        "vanilla `setBlock(pos, air, 11)` notifies neighbours, so the lily pad loses its support"
    );
    let dropped = dropped_items(
        &fixture.world,
        DVec3::new(
            f64::from(above.x()) + 0.5,
            f64::from(above.y()) + 0.5,
            f64::from(above.z()) + 0.5,
        ),
    );
    assert!(
        dropped.iter().any(|entity| entity
            .downcast_ref::<ItemEntity>()
            .is_some_and(|item| item.get_item().is(&vanilla_items::LILY_PAD))),
        "the unsupported lily pad drops its item"
    );
}

#[test]
fn a_dispensed_bucket_over_nothing_is_thrown_out() {
    let (fixture, pos) = pulse_empty_bucket("dispenser_bucket_nothing");

    assert_eq!(
        fixture.world.get_block_state(TARGET),
        vanilla_blocks::AIR.default_state()
    );
    assert!(item_at(&fixture, pos, 0).is_empty());
    assert_eq!(
        ejected_items(&fixture, pos).len(),
        1,
        "vanilla `DefaultDispenseItemBehavior` throws out a bucket that found no fluid"
    );
}
