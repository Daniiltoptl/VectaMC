use super::*;
use crate::entity::AddEntityError;

#[test]
fn rejected_projectile_insertion_does_not_cancel_consumption_and_level_events() {
    let (fixture, _, _player, packets) =
        listening_dispenser_fixture("dispenser_projectile_rejection", Direction::East);
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
    let rejected = Arc::new(SnowballEntity::new(
        &vanilla_entities::SNOWBALL,
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
        ItemStack::with_count(&vanilla_items::SNOWBALL, 2),
    );
    packets.lock().clear();

    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );

    assert_eq!(item_at(&fixture, pos, 0).count(), 1);
    assert!(projectiles_of_type(&fixture, pos, &vanilla_entities::SNOWBALL).is_empty());
    assert_eq!(
        sent_level_events(&packets),
        [
            (SOUND_DISPENSER_PROJECTILE_LAUNCH, pos, 0),
            (
                PARTICLES_SHOOT_SMOKE,
                pos,
                Direction::East.get_3d_data_value()
            ),
        ]
    );
}
