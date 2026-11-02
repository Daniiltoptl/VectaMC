use super::*;

use text_components::TextComponent;
use vecta_registry::data_components::PotionContents;
use vecta_registry::data_components::vanilla_components::{
    CUSTOM_NAME, POTION_CONTENTS, POTION_DURATION_SCALE,
};
use vecta_registry::items::ItemRef;
use vecta_registry::vanilla_potions;
use vecta_utils::DowncastType;

use crate::chunk_saver::{ChunkStorage, PersistentEntity};
use crate::entity::entities::{SplashPotionEntity, ThrownEggEntity, ThrownExperienceBottleEntity};
use crate::entity::{Entity, ThrowableItemProjectile};

fn assert_dispensed_round_trip<E>(name: &'static str, item: ItemRef, entity_type: EntityTypeRef)
where
    E: Entity + ThrowableItemProjectile + DowncastType,
{
    let (fixture, pos) = dispenser_fixture(name, Direction::South);
    let mut stack = ItemStack::new(item);
    stack.set(CUSTOM_NAME, TextComponent::plain("Saved projectile"));
    if item == &*vanilla_items::SPLASH_POTION {
        stack.set(
            POTION_CONTENTS,
            PotionContents::of(&vanilla_potions::SWIFTNESS),
        );
        stack.set(POTION_DURATION_SCALE, 1.75);
    }
    let expected_item = stack.copy_with_count(1);
    set_item_at(&fixture, pos, 0, stack);
    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );

    let projectiles = projectiles_of_type(&fixture, pos, entity_type);
    assert_eq!(projectiles.len(), 1);
    let original = &projectiles[0];
    original.set_rotation((37.0, -12.0));

    let persistent = ChunkStorage::entity_tree_to_persistent(original)
        .expect("a flying dispensed projectile is persistent");
    let bytes = wincode::serialize(&persistent).expect("the entity record encodes");
    let decoded: PersistentEntity =
        wincode::deserialize_exact(&bytes).expect("the entity record decodes");
    let restored = ChunkStorage::persistent_to_entity_tree_at_level(
        &decoded,
        ChunkPos::from_entity_pos(original.position()),
        &Arc::downgrade(&fixture.world),
    );

    assert_eq!(restored.len(), 1, "the registered entity factory loads it");
    let loaded = &restored[0];
    assert_eq!(loaded.uuid(), original.uuid());
    assert_eq!(loaded.position(), original.position());
    assert_eq!(loaded.velocity(), original.velocity());
    assert_eq!(loaded.rotation(), original.rotation());
    assert!(
        loaded
            .as_projectile()
            .expect("the factory restores a projectile")
            .projectile_owner()
            .is_none()
    );
    assert_eq!(
        loaded
            .downcast_ref::<E>()
            .expect("the factory restores the concrete projectile")
            .get_item(),
        expected_item,
        "item identity, count and components survive the storage codec and entity factory"
    );
}

#[test]
fn dispensed_consumables_restore_flight_and_components_through_persistent_records() {
    assert_dispensed_round_trip::<ThrownExperienceBottleEntity>(
        "dispenser_bottle_persistent",
        &vanilla_items::EXPERIENCE_BOTTLE,
        &vanilla_entities::EXPERIENCE_BOTTLE,
    );
    for (name, item) in [
        ("dispenser_egg_persistent", &vanilla_items::EGG),
        ("dispenser_blue_egg_persistent", &vanilla_items::BLUE_EGG),
        ("dispenser_brown_egg_persistent", &vanilla_items::BROWN_EGG),
    ] {
        assert_dispensed_round_trip::<ThrownEggEntity>(name, item, &vanilla_entities::EGG);
    }
    assert_dispensed_round_trip::<SplashPotionEntity>(
        "dispenser_potion_persistent",
        &vanilla_items::SPLASH_POTION,
        &vanilla_entities::SPLASH_POTION,
    );
}
