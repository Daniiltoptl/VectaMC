use std::io::Cursor;
use std::sync::{Arc, Weak};

use glam::DVec3;
use simdnbt::borrow::read_compound;
use simdnbt::owned::NbtCompound;
use steel_registry::data_components::vanilla_components::CUSTOM_NAME;
use steel_registry::item_stack::ItemStack;
use steel_registry::{init_vanilla_registry, vanilla_entities, vanilla_items};
use steel_utils::{ChunkPos, Direction, Downcast as _, WorldAabb};
use text_components::TextComponent;

use crate::behavior::init_behaviors;
use crate::entity::entities::{ExperienceOrbEntity, PigEntity};
use crate::entity::{
    Entity, EntityHitResult, LivingEntity, Projectile, ProjectileHit, SharedEntity,
    ThrowableItemProjectile,
};
use crate::test_support::{fresh_test_world, insert_ready_full_chunk};
use crate::world::{ClipHitResult, World};

use super::ThrownExperienceBottleEntity;

#[test]
fn block_impact_awards_experience_toward_each_hit_face_and_discards_bottle() {
    init_vanilla_registry();
    init_behaviors();
    for direction in Direction::ALL {
        let fixture = fresh_test_world("experience_bottle_block_hit");
        let world = &fixture.world;
        let position = DVec3::new(0.5, 80.5, 0.5);
        insert_ready_full_chunk(world, ChunkPos::from_entity_pos(position));
        let bottle = Arc::new(ThrownExperienceBottleEntity::new(
            &vanilla_entities::EXPERIENCE_BOTTLE,
            1,
            position,
            Arc::downgrade(world),
        ));
        let hit = ProjectileHit::Block {
            location: position,
            hit: ClipHitResult {
                location: position,
                direction,
                block_pos: steel_utils::BlockPos::new(0, 80, 0),
                miss: false,
                inside: false,
                world_border_hit: false,
            },
        };
        Arc::clone(&bottle).on_hit(&hit);
        assert!(bottle.is_removed());
        let entities = world.get_entities_in_aabb(&WorldAabb::of_size(position, 4.0, 4.0, 4.0));
        let mut total = 0;
        let normal = direction.offset_vec().as_dvec3();
        for entity in entities {
            let orb = entity
                .downcast_ref::<ExperienceOrbEntity>()
                .expect("only XP orbs spawned");
            total += orb.value() * orb.count();
            assert!((orb.position() - position).dot(normal) > 0.0);
            assert!(orb.velocity().dot(normal) >= 0.0);
        }
        assert!((3..=11).contains(&total));
    }
}

#[test]
fn entity_impact_does_not_deal_damage_and_orbs_face_against_bottle_motion() {
    init_vanilla_registry();
    init_behaviors();
    let fixture = fresh_test_world("experience_bottle_entity_hit");
    let world = &fixture.world;
    let position = DVec3::new(0.5, 80.5, 0.5);
    insert_ready_full_chunk(world, ChunkPos::from_entity_pos(position));
    let bottle = Arc::new(ThrownExperienceBottleEntity::new(
        &vanilla_entities::EXPERIENCE_BOTTLE,
        1,
        position,
        Arc::downgrade(world),
    ));
    bottle.set_velocity(DVec3::new(0.4, -0.2, 0.3));
    let target = Arc::new(PigEntity::new(
        &vanilla_entities::PIG,
        2,
        position,
        Arc::downgrade(world),
    ));
    let health = target.get_health();
    let entity: SharedEntity = Arc::<PigEntity>::clone(&target);
    Arc::clone(&bottle).on_hit(&ProjectileHit::Entity(EntityHitResult {
        entity,
        location: position,
    }));
    assert_eq!(target.get_health(), health);
    assert!(bottle.is_removed());
    let entities = world.get_entities_in_aabb(&WorldAabb::of_size(position, 4.0, 4.0, 4.0));
    let mut total = 0;
    for entity in entities {
        let orb = entity
            .downcast_ref::<ExperienceOrbEntity>()
            .expect("experience orb");
        total += orb.value() * orb.count();
        assert!(orb.velocity().dot(-bottle.velocity()) >= 0.0);
    }
    assert!((3..=11).contains(&total));
}

#[test]
fn saved_bottle_preserves_shooter_and_rendered_item_components() {
    init_vanilla_registry();
    let bottle = ThrownExperienceBottleEntity::new(
        &vanilla_entities::EXPERIENCE_BOTTLE,
        1,
        DVec3::ZERO,
        Weak::<World>::new(),
    );
    let owner = uuid::Uuid::new_v4();
    bottle.set_owner_uuid(Some(owner));
    let mut item = ItemStack::with_count(&vanilla_items::EXPERIENCE_BOTTLE, 4);
    item.set(CUSTOM_NAME, TextComponent::plain("Bottled experience"));
    bottle.set_item_clamped(item);
    let mut nbt = NbtCompound::new();
    bottle.save_additional(&mut nbt);
    let mut bytes = Vec::new();
    nbt.write(&mut bytes);
    let borrowed = read_compound(&mut Cursor::new(&bytes)).expect("saved bottle NBT");
    let loaded = ThrownExperienceBottleEntity::new(
        &vanilla_entities::EXPERIENCE_BOTTLE,
        2,
        DVec3::ZERO,
        Weak::<World>::new(),
    );
    loaded.load_additional((&borrowed).into());
    assert_eq!(loaded.projectile_owner_uuid(), Some(owner));
    assert_eq!(loaded.get_item(), bottle.get_item());
    assert_eq!(loaded.get_item().count(), 1);
}
