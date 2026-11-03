use super::*;
use crate::entity::{ENTITY_LOAD_MAX_HORIZONTAL_POSITION, ENTITY_LOAD_MAX_VERTICAL_POSITION};
use vecta_registry::RegistryEntry as _;

fn test_persistent_end_crystal(pos: DVec3) -> PersistentEntity {
    PersistentEntity {
        entity_type: vanilla_entities::END_CRYSTAL.key.clone(),
        uuid: [9; 16],
        pos: [pos.x, pos.y, pos.z],
        motion: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0],
        fall_distance: 0.0,
        remaining_fire_ticks: 0,
        ticks_frozen: 0,
        is_in_powder_snow: false,
        was_in_powder_snow: false,
        has_visual_fire: false,
        on_ground: false,
        no_gravity: false,
        invulnerable: false,
        air_supply: DEFAULT_MAX_AIR_SUPPLY,
        portal_cooldown: 0,
        custom_name_nbt: Vec::new(),
        custom_name_visible: false,
        silent: false,
        glowing: false,
        tags: Vec::new(),
        custom_data_nbt: Vec::new(),
        nbt_data: Vec::new(),
        passengers: Vec::new(),
    }
}

#[test]
fn persistent_entity_load_clamps_position_like_vanilla() {
    init_globals();

    let persistent =
        test_persistent_end_crystal(DVec3::new(100_000_000.0, -100_000_000.0, -100_000_000.0));
    let Some(entity) =
        ChunkStorage::persistent_to_entity_at_level(&persistent, ChunkPos::new(0, 0), &Weak::new())
    else {
        panic!("entity should load with clamped position");
    };

    assert_eq!(
        entity.position(),
        DVec3::new(
            ENTITY_LOAD_MAX_HORIZONTAL_POSITION,
            -ENTITY_LOAD_MAX_VERTICAL_POSITION,
            -ENTITY_LOAD_MAX_HORIZONTAL_POSITION,
        )
    );
    assert_eq!(entity.base().old_position(), entity.position());
    assert_eq!(entity.base().old_rotation(), entity.rotation());
}

#[test]
fn persistent_entity_load_rejects_non_finite_rotation_like_vanilla() {
    init_globals();

    let mut persistent = test_persistent_end_crystal(DVec3::new(1.0, 2.0, 3.0));
    persistent.rotation = [f32::NAN, 0.0];

    assert!(
            ChunkStorage::persistent_to_entity_at_level(
                &persistent,
                ChunkPos::new(0, 0),
                &Weak::new(),
            )
            .is_none()
        );
}

#[test]
fn proto_block_entities_roundtrip_and_promote_to_full_chunk() {
    init_globals();

    let pos = ChunkPos::new(0, 0);
    let block_pos = BlockPos::new(3, 4, 5);
    let proto = Chunk::new(single_empty_section(), pos, 0, 16, Weak::new());
    let barrel = REGISTRY
        .blocks
        .get_default_state_id(&vanilla_blocks::BARREL);
    proto.set_block_state_for_generation(
        ChunkStatus::Features,
        block_pos,
        barrel,
        UpdateFlags::UPDATE_NONE,
    );
    proto.set_pending_block_entity(block_pos);

    assert!(proto.get_block_entity(block_pos).is_none());
    assert_eq!(proto.pending_block_entity_positions(), [block_pos]);

    let chunk = proto;
    let Some(prepared) =
        ChunkStorage::prepare_chunk_save(&chunk, ChunkStatus::Features, &[], false)
    else {
        panic!("dirty proto chunk should prepare for saving");
    };
    assert_eq!(prepared.persistent.block_entities.len(), 1);
    assert!(prepared.persistent.block_entities[0].entity_type.is_none());

    let loaded = ChunkStorage::persistent_to_chunk(
        &prepared.persistent,
        pos,
        ChunkStatus::Features,
        0,
        16,
        Weak::new(),
    );
    let loaded_proto = loaded.chunk;
    assert!(loaded_proto.get_block_entity(block_pos).is_none());
    assert_eq!(loaded_proto.pending_block_entity_positions(), [block_pos]);

    let full_ref = loaded_proto.promote_to_full().chunk;
    assert!(full_ref.get_block_entities().is_empty());
    assert_eq!(full_ref.pending_block_entity_positions(), [block_pos]);

    let Some(full_save) =
        ChunkStorage::prepare_chunk_save(&loaded_proto, ChunkStatus::Full, &[], true)
    else {
        panic!("forced full-chunk save should retain the pending marker");
    };
    assert_eq!(full_save.persistent.block_entities.len(), 1);
    assert!(full_save.persistent.block_entities[0].entity_type.is_none());
    let loaded = ChunkStorage::persistent_to_chunk(
        &full_save.persistent,
        pos,
        ChunkStatus::Full,
        0,
        16,
        Weak::new(),
    );
    let loaded_full = loaded.chunk;
    let loaded_full = FullChunkRef::from_full_context(&loaded_full);
    assert!(loaded_full.get_block_entities().is_empty());
    assert_eq!(loaded_full.pending_block_entity_positions(), [block_pos]);
    assert!(loaded_full.get_block_entity(block_pos).is_some());
    assert_eq!(loaded_full.pending_block_entity_positions().len(), 0);
}

#[test]
fn persistent_block_entity_with_invalid_live_state_is_rejected_before_construction() {
    init_globals();
    let persistent = PersistentBlockEntity {
        x: 1,
        y: 2,
        z: 3,
        entity_type: Some(vanilla_block_entity_types::BARREL.key.clone()),
        nbt_data: Vec::new(),
    };

    assert!(
        ChunkStorage::persistent_to_block_entity_at(
            &persistent,
            BlockPos::new(1, 2, 3),
            Weak::new(),
            vanilla_blocks::STONE.default_state(),
        )
        .is_none()
    );
}

#[test]
fn persistent_block_entity_with_malformed_nbt_is_dropped() {
    init_globals();
    let persistent = PersistentBlockEntity {
        x: 1,
        y: 2,
        z: 3,
        entity_type: Some(vanilla_block_entity_types::BARREL.key.clone()),
        nbt_data: vec![0xff],
    };

    assert!(
        ChunkStorage::persistent_to_block_entity_at(
            &persistent,
            BlockPos::new(1, 2, 3),
            Weak::new(),
            vanilla_blocks::BARREL.default_state(),
        )
        .is_none()
    );
}

#[test]
fn proto_entities_roundtrip_and_promote_to_full_chunk() {
    init_globals();

    let pos = ChunkPos::new(0, 0);
    let entity_pos = DVec3::new(5.5, 6.0, 7.5);
    let proto = Chunk::new(single_empty_section(), pos, 0, 16, Weak::new());
    let crystal = Arc::new(EndCrystalEntity::new(
        &vanilla_entities::END_CRYSTAL,
        next_entity_id(),
        entity_pos,
        Weak::new(),
    ));
    crystal.set_beam_target(Some(BlockPos::new(0, 64, 0)));
    crystal.set_invulnerable(true);
    crystal.set_fall_distance(3.75);
    crystal.set_no_gravity(true);
    crystal.set_air_supply(120);
    crystal.set_portal_cooldown(9);
    crystal.set_custom_name(Some(TextComponent::plain("End Test")));
    crystal.set_custom_name_visible(true);
    crystal.set_silent(true);
    crystal.set_glowing_tag(true);
    assert!(crystal.add_tag("vecta:test".to_owned()));
    let mut custom_data = NbtCompound::new();
    custom_data.insert("marker", "roundtrip");
    crystal.set_custom_data(custom_data);
    proto.add_entity(crystal);

    let chunk = proto;
    let Some(prepared) =
        ChunkStorage::prepare_chunk_save(&chunk, ChunkStatus::Features, &[], false)
    else {
        panic!("dirty proto chunk should prepare for saving");
    };
    assert_eq!(prepared.persistent.entities.len(), 1);
    assert!((prepared.persistent.entities[0].fall_distance - 3.75).abs() <= f64::EPSILON);
    assert!(prepared.persistent.entities[0].no_gravity);
    assert!(prepared.persistent.entities[0].invulnerable);
    assert_eq!(prepared.persistent.entities[0].air_supply, 120);
    assert_eq!(prepared.persistent.entities[0].portal_cooldown, 9);
    assert!(prepared.persistent.entities[0].custom_name_visible);
    assert!(prepared.persistent.entities[0].silent);
    assert!(prepared.persistent.entities[0].glowing);
    assert_eq!(
        prepared.persistent.entities[0].tags,
        vec!["vecta:test".to_owned()]
    );
    assert_ne!(prepared.persistent.entities[0].custom_name_nbt.len(), 0);
    assert_ne!(prepared.persistent.entities[0].custom_data_nbt.len(), 0);
    let custom_name_nbt = read_borrowed_compound(&mut Cursor::new(
        &prepared.persistent.entities[0].custom_name_nbt,
    ))
    .expect("saved custom name should be valid NBT");
    let custom_name_nbt = simdnbt::borrow::NbtCompound::from(&custom_name_nbt);
    assert_eq!(
        custom_name_nbt
            .string("CustomName")
            .map(|value| value.to_str().into_owned()),
        Some("End Test".to_owned())
    );

    let loaded = ChunkStorage::persistent_to_chunk(
        &prepared.persistent,
        pos,
        ChunkStatus::Features,
        0,
        16,
        Weak::new(),
    );
    assert!(loaded.pending_entities.is_empty());
    let loaded_proto = loaded.chunk;
    assert_eq!(loaded_proto.get_entities().len(), 1);

    let promoted = loaded_proto.promote_to_full();
    assert_eq!(promoted.pending_entities.len(), 1);
    assert!(promoted.pending_entities[0].is_no_gravity());
    assert!(promoted.pending_entities[0].is_invulnerable());
    assert_eq!(promoted.pending_entities[0].air_supply(), 120);
    assert_eq!(promoted.pending_entities[0].portal_cooldown(), 9);
    assert_eq!(
        promoted.pending_entities[0].custom_name(),
        Some(TextComponent::plain("End Test"))
    );
    assert!(promoted.pending_entities[0].is_custom_name_visible());
    assert!(promoted.pending_entities[0].is_silent());
    assert!(promoted.pending_entities[0].has_glowing_tag());
    assert_eq!(
        promoted.pending_entities[0].tags(),
        vec!["vecta:test".to_owned()]
    );
    assert_eq!(
        promoted.pending_entities[0]
            .custom_data()
            .string("marker")
            .map(ToString::to_string),
        Some("roundtrip".to_owned())
    );
}

#[test]
fn unsupported_persistent_entity_is_dropped_after_load_and_resave() {
    init_globals();

    let pos = ChunkPos::new(0, 0);
    let proto = Chunk::new(single_empty_section(), pos, 0, 16, Weak::new());
    let Some(mut prepared) =
        ChunkStorage::prepare_chunk_save(&proto, ChunkStatus::Features, &[], true)
    else {
        panic!("forced proto chunk save should prepare");
    };
    let mut unsupported = test_persistent_end_crystal(DVec3::new(5.5, 6.0, 7.5));
    unsupported.entity_type = vanilla_entities::VILLAGER.key.clone();
    prepared.persistent.entities.push(unsupported);

    let loaded = ChunkStorage::persistent_to_chunk(
        &prepared.persistent,
        pos,
        ChunkStatus::Features,
        0,
        16,
        Weak::new(),
    );
    assert!(loaded.pending_entities.is_empty());
    assert!(loaded.chunk.get_entities().is_empty());

    let Some(resaved) =
        ChunkStorage::prepare_chunk_save(&loaded.chunk, ChunkStatus::Features, &[], true)
    else {
        panic!("forced loaded chunk save should prepare");
    };
    assert!(resaved.persistent.entities.is_empty());
}

#[test]
fn prepared_save_reports_handled_runtime_entity_ids() {
    init_globals();

    let pos = ChunkPos::new(0, 0);
    let proto = Chunk::new(single_empty_section(), pos, 0, 16, Weak::new());
    let chunk = proto;
    let entity: SharedEntity = Arc::new(EndCrystalEntity::new(
        &vanilla_entities::END_CRYSTAL,
        next_entity_id(),
        DVec3::new(5.5, 6.0, 7.5),
        Weak::new(),
    ));

    let Some(prepared) = ChunkStorage::prepare_chunk_save(
        &chunk,
        ChunkStatus::Features,
        slice::from_ref(&entity),
        true,
    ) else {
        panic!("forced runtime entity save should prepare a chunk save");
    };

    assert_eq!(prepared.handled_runtime_entity_ids, vec![entity.id()]);
    assert_eq!(prepared.persistent.entities.len(), 1);
}

#[test]
fn full_chunk_load_defers_entities_to_world_registration() {
    init_globals();

    let pos = ChunkPos::new(0, 0);
    let proto = Chunk::new(single_empty_section(), pos, 0, 16, Weak::new());
    let chunk = proto;
    let entity: SharedEntity = Arc::new(EndCrystalEntity::new(
        &vanilla_entities::END_CRYSTAL,
        next_entity_id(),
        DVec3::new(5.5, 6.0, 7.5),
        Weak::new(),
    ));

    let Some(prepared) = ChunkStorage::prepare_chunk_save(
        &chunk,
        ChunkStatus::Features,
        slice::from_ref(&entity),
        true,
    ) else {
        panic!("forced runtime entity save should prepare a chunk save");
    };

    let loaded = ChunkStorage::persistent_to_chunk(
        &prepared.persistent,
        pos,
        ChunkStatus::Full,
        0,
        16,
        Weak::new(),
    );

    assert_eq!(loaded.status, ChunkStatus::Full);
    assert_eq!(loaded.pending_entities.len(), 1);
    assert_eq!(loaded.pending_entities[0].uuid(), entity.uuid());
}

#[test]
fn runtime_entity_passengers_save_nested_and_load_flattened_for_registration() {
    init_globals();

    let pos = ChunkPos::new(0, 0);
    let proto = Chunk::new(single_empty_section(), pos, 0, 16, Weak::new());
    let chunk = proto;
    let vehicle: SharedEntity = Arc::new(EndCrystalEntity::new(
        &vanilla_entities::END_CRYSTAL,
        next_entity_id(),
        DVec3::new(5.5, 6.0, 7.5),
        Weak::new(),
    ));
    let passenger: SharedEntity = Arc::new(EndCrystalEntity::new(
        &vanilla_entities::END_CRYSTAL,
        next_entity_id(),
        DVec3::new(5.5, 8.0, 7.5),
        Weak::new(),
    ));
    EntityBase::restore_passenger_relationship(&vehicle, &passenger);
    let vehicle_uuid = vehicle.uuid();
    let passenger_uuid = passenger.uuid();
    let entities = [Arc::clone(&vehicle), Arc::clone(&passenger)];

    let Some(prepared) =
        ChunkStorage::prepare_chunk_save(&chunk, ChunkStatus::Features, &entities, true)
    else {
        panic!("forced runtime entity save should prepare a chunk save");
    };

    assert_eq!(prepared.persistent.entities.len(), 1);
    assert_eq!(
        prepared.persistent.entities[0].uuid,
        *vehicle_uuid.as_bytes()
    );
    assert_eq!(prepared.persistent.entities[0].passengers.len(), 1);
    assert_eq!(
        prepared.persistent.entities[0].passengers[0].uuid,
        *passenger_uuid.as_bytes()
    );

    let loaded = ChunkStorage::persistent_to_chunk(
        &prepared.persistent,
        pos,
        ChunkStatus::Full,
        0,
        16,
        Weak::new(),
    );

    assert_eq!(loaded.status, ChunkStatus::Full);
    assert_eq!(loaded.pending_entities.len(), 2);
    let Some(loaded_passenger) = loaded
        .pending_entities
        .iter()
        .find(|entity| entity.uuid() == passenger_uuid)
    else {
        panic!("passenger should load into pending registration list");
    };
    let Some(loaded_vehicle) = loaded_passenger.vehicle() else {
        panic!("passenger should restore its vehicle relationship");
    };
    assert_eq!(loaded_vehicle.uuid(), vehicle_uuid);
    assert!(loaded_vehicle.has_passenger(loaded_passenger.as_ref()));
}

#[test]
fn runtime_entity_passengers_skip_non_serializable_entities_like_vanilla() {
    init_globals();

    let pos = ChunkPos::new(0, 0);
    let proto = Chunk::new(single_empty_section(), pos, 0, 16, Weak::new());
    let chunk = proto;
    let vehicle: SharedEntity = Arc::new(EndCrystalEntity::new(
        &vanilla_entities::END_CRYSTAL,
        next_entity_id(),
        DVec3::new(5.5, 6.0, 7.5),
        Weak::new(),
    ));
    let passenger: SharedEntity = TestEntity::shared(
        next_entity_id(),
        DVec3::new(5.5, 8.0, 7.5),
        Weak::new(),
        &vanilla_entities::PLAYER,
    );
    EntityBase::restore_passenger_relationship(&vehicle, &passenger);
    let vehicle_uuid = vehicle.uuid();

    let Some(prepared) = ChunkStorage::prepare_chunk_save(
        &chunk,
        ChunkStatus::Features,
        slice::from_ref(&vehicle),
        true,
    ) else {
        panic!("forced runtime entity save should prepare a chunk save");
    };

    assert_eq!(prepared.persistent.entities.len(), 1);
    assert_eq!(
        prepared.persistent.entities[0].uuid,
        *vehicle_uuid.as_bytes()
    );
    assert!(prepared.persistent.entities[0].passengers.is_empty());

    let loaded = ChunkStorage::persistent_to_chunk(
        &prepared.persistent,
        pos,
        ChunkStatus::Full,
        0,
        16,
        Weak::new(),
    );

    assert_eq!(loaded.status, ChunkStatus::Full);
    assert_eq!(loaded.pending_entities.len(), 1);
    assert_eq!(loaded.pending_entities[0].uuid(), vehicle_uuid);
}

#[test]
fn unimplemented_block_entities_preserve_nbt_through_proto_save_load() {
    init_globals();

    let pos = ChunkPos::new(0, 0);
    let block_pos = BlockPos::new(4, 4, 6);
    let proto = Chunk::new(single_empty_section(), pos, 0, 16, Weak::new());
    let spawner = REGISTRY
        .blocks
        .get_default_state_id(&vanilla_blocks::SPAWNER);
    proto.set_block_state_for_generation(
        ChunkStatus::Features,
        block_pos,
        spawner,
        UpdateFlags::UPDATE_NONE,
    );

    let mut nbt = NbtCompound::new();
    nbt.insert("LootTable", "minecraft:chests/simple_dungeon");
    nbt.insert("LootTableSeed", 42_i64);
    let entity = BLOCK_ENTITIES.create_and_load_owned_or_unimplemented(
        &vanilla_block_entity_types::MOB_SPAWNER,
        proto.level_weak(),
        block_pos,
        spawner,
        nbt,
    );
    assert!(proto.set_block_entity(entity));

    let chunk = proto;
    let Some(prepared) =
        ChunkStorage::prepare_chunk_save(&chunk, ChunkStatus::Features, &[], false)
    else {
        panic!("dirty proto chunk should prepare for saving");
    };
    assert_eq!(prepared.persistent.block_entities.len(), 1);

    let loaded = ChunkStorage::persistent_to_chunk(
        &prepared.persistent,
        pos,
        ChunkStatus::Features,
        0,
        16,
        Weak::new(),
    );
    let loaded_proto = loaded.chunk;
    let Some(loaded_entity) = loaded_proto.get_block_entity(block_pos) else {
        panic!("unimplemented block entity should survive chunk load");
    };

    let mut saved = NbtCompound::new();
    assert_eq!(
        loaded_entity.get_type().id(),
        vanilla_block_entity_types::MOB_SPAWNER.id()
    );
    loaded_entity.save_additional(&mut saved);

    assert_eq!(
        saved.string("LootTable").map(ToString::to_string),
        Some("minecraft:chests/simple_dungeon".to_owned())
    );
    assert_eq!(saved.long("LootTableSeed"), Some(42));
}

#[test]
fn a_saved_barrel_restores_identity_and_contents_through_the_storage_decoder() {
    use std::slice;
    use std::sync::Weak;

    use simdnbt::owned::NbtCompound;
    use text_components::TextComponent;
    use vecta_registry::item_stack::ItemStack;
    use vecta_registry::{vanilla_block_entity_types, vanilla_blocks, vanilla_items};

    use vecta_utils::Downcast as _;

    use crate::block_entity::entities::BarrelBlockEntity;
    use crate::block_entity::{BlockEntity as _, BlockEntityLoadContext};
    use crate::inventory::lock::ContainerLockGuard;

    init_globals();
    let pos = BlockPos::new(3, 64, 5);
    let state = vanilla_blocks::BARREL.default_state();
    let barrel = BarrelBlockEntity::new(Weak::new(), pos, state);
    let mut identity = NbtCompound::new();
    identity.insert("CustomName", TextComponent::plain("Storage").to_codec_nbt());
    let mut lock = NbtCompound::new();
    lock.insert("items", "minecraft:diamond");
    identity.insert("lock", lock);
    assert!(barrel.load_item_custom_data(&identity, &BlockEntityLoadContext::detached()));
    let container = barrel.container_ref().expect("barrel container");
    let id = container.container_id();
    {
        let mut guard = ContainerLockGuard::lock_all(slice::from_ref(&container));
        assert!(guard.set_item(id, 7, ItemStack::with_count(&vanilla_items::DIAMOND, 5)));
    }

    let mut nbt_bytes = Vec::new();
    barrel.save_without_metadata().write(&mut nbt_bytes);
    let persistent = PersistentBlockEntity {
        x: 3,
        y: 64,
        z: 5,
        entity_type: Some(vanilla_block_entity_types::BARREL.key.clone()),
        nbt_data: nbt_bytes,
    };

    let restored =
        ChunkStorage::persistent_to_block_entity_at(&persistent, pos, Weak::new(), state)
            .expect("the storage decoder rebuilds the barrel");
    let restored = restored
        .downcast_ref::<BarrelBlockEntity>()
        .expect("the decoder uses the registered barrel factory");
    assert_eq!(
        restored.identity().custom_name(),
        Some(TextComponent::plain("Storage"))
    );
    assert!(restored.identity().is_locked());

    let restored_container = restored.container_ref().expect("restored container");
    let restored_id = restored_container.container_id();
    let guard = ContainerLockGuard::lock_all(slice::from_ref(&restored_container));
    assert_eq!(
        guard
            .get(restored_id)
            .expect("the restored container is locked")
            .get_item(7),
        &ItemStack::with_count(&vanilla_items::DIAMOND, 5),
        "the restored container is usable and still holds its contents"
    );
    drop(guard);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one test walks the whole write/read path so the region file assertions stay together"
)]
fn a_barrel_survives_a_real_region_file_round_trip() {
    use std::env::temp_dir;
    use std::slice;
    use std::sync::Weak;
    use std::time::{SystemTime, UNIX_EPOCH};

    use tokio::runtime::Builder;

    use simdnbt::owned::NbtCompound;
    use text_components::TextComponent;
    use vecta_registry::data_components::DataComponentMap;
    use vecta_registry::data_components::components::CustomData;
    use vecta_registry::data_components::vanilla_components::CUSTOM_DATA;
    use vecta_registry::item_stack::ItemStack;
    use vecta_registry::{vanilla_blocks, vanilla_items};
    use vecta_utils::Downcast as _;

    use crate::block_entity::entities::BarrelBlockEntity;
    use crate::block_entity::{BlockEntity as _, BlockEntityLoadContext};
    use crate::chunk::status::ChunkStatus;
    use crate::chunk_saver::region_manager::RegionManager;
    use crate::inventory::lock::ContainerLockGuard;
    use crate::test_support::{fresh_test_world, insert_ready_full_chunk};
    use std::fs;

    init_globals();
    let fixture = fresh_test_world("region_barrel_roundtrip");
    let world = &fixture.world;
    let pos = BlockPos::new(3, 64, 5);
    let chunk_pos = ChunkPos::from_block_pos(pos);
    insert_ready_full_chunk(world, chunk_pos);
    assert!(world.set_block(
        pos,
        vanilla_blocks::BARREL.default_state(),
        UpdateFlags::UPDATE_ALL
    ));

    let block_entity = world.get_block_entity(pos).expect("barrel entity");
    let mut identity = NbtCompound::new();
    identity.insert("CustomName", TextComponent::plain("Storage").to_codec_nbt());
    let mut lock = NbtCompound::new();
    lock.insert("items", "minecraft:diamond");
    identity.insert("lock", lock);
    assert!(block_entity.load_item_custom_data(&identity, &BlockEntityLoadContext::detached()));
    let barrel = block_entity
        .downcast_ref::<BarrelBlockEntity>()
        .expect("barrel implementation");
    let container = barrel.container_ref().expect("barrel container");
    let id = container.container_id();
    {
        let mut guard = ContainerLockGuard::lock_all(slice::from_ref(&container));
        assert!(guard.set_item(id, 7, ItemStack::with_count(&vanilla_items::DIAMOND, 5)));
    }
    let mut stored = DataComponentMap::new();
    let mut custom = NbtCompound::new();
    custom.insert("marker", 1_i32);
    stored.set(
        CUSTOM_DATA,
        Some(CustomData::try_from_compound(custom).expect("custom data")),
    );
    block_entity.base().set_stored_components(stored);

    let prepared = world
        .chunk_map
        .with_chunk_at_status(chunk_pos, ChunkStatus::Full, |chunk| {
            ChunkStorage::prepare_chunk_save(chunk, ChunkStatus::Full, &[], true)
        })
        .flatten()
        .expect("the dirty chunk prepares for saving");

    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime");

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    let dir = temp_dir().join(format!("vecta-region-{unique}"));
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("encoding pool");
    let regions = RegionManager::new(dir.clone());
    let loaded = runtime.block_on(async {
        assert!(
            regions
                .save_chunk_data(prepared, &pool)
                .await
                .expect("region write"),
            "the prepared chunk is written to the region file"
        );

        // The load path expects the region to be acquired, like the chunk map does.
        assert!(
            regions
                .acquire_chunk(chunk_pos)
                .await
                .expect("region acquire"),
            "the saved chunk is present in the region index"
        );
        let storage = ChunkStorage::Disk(regions);
        storage
            .load_chunk(
                chunk_pos,
                world.dimension_type.min_y,
                world.dimension_type.height,
                Weak::new(),
                &pool,
            )
            .await
            .expect("region read")
            .expect("the region file holds the chunk")
    });
    let restored = loaded
        .chunk
        .get_block_entity(pos)
        .expect("the loaded chunk rebuilds the barrel");
    let restored_barrel = restored
        .downcast_ref::<BarrelBlockEntity>()
        .expect("the region decoder uses the registered barrel factory");
    assert_eq!(
        restored_barrel.identity().custom_name(),
        Some(TextComponent::plain("Storage")),
        "the name survives the region file"
    );
    assert!(restored_barrel.identity().is_locked());

    let restored_container = restored_barrel.container_ref().expect("restored container");
    let restored_id = restored_container.container_id();
    let guard = ContainerLockGuard::lock_all(slice::from_ref(&restored_container));
    assert_eq!(
        guard
            .get(restored_id)
            .expect("the restored container is locked")
            .get_item(7),
        &ItemStack::with_count(&vanilla_items::DIAMOND, 5),
        "the restored container is functional and still holds its contents"
    );
    drop(guard);
    assert!(
        restored
            .base()
            .stored_components()
            .get(CUSTOM_DATA)
            .is_some(),
        "an explicit component survives the region file"
    );

    fs::remove_dir_all(dir).expect("temporary region directory is removed");
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one test walks the whole write/read path so the living-state assertions stay together"
)]
fn a_living_entity_keeps_health_effects_and_equipment_across_a_region_file() {
    use std::env::temp_dir;
    use std::fs;
    use std::slice;
    use std::sync::Weak;
    use std::time::{SystemTime, UNIX_EPOCH};

    use tokio::runtime::Builder;
    use vecta_registry::item_stack::ItemStack;
    use vecta_registry::{vanilla_items, vanilla_mob_effects};

    use crate::chunk::status::ChunkStatus;
    use crate::chunk_saver::region_manager::RegionManager;
    use crate::entity::entities::SheepEntity;
    use crate::entity::{MobEffectInstance, SharedEntity};
    use crate::inventory::equipment::EquipmentSlot;
    use crate::test_support::{fresh_test_world, insert_ready_full_chunk};

    init_globals();
    let fixture = fresh_test_world("region_sheep_roundtrip");
    let world = &fixture.world;
    let pos = BlockPos::new(4, 65, 6);
    let chunk_pos = ChunkPos::from_block_pos(pos);
    insert_ready_full_chunk(world, chunk_pos);

    let sheep: SharedEntity = Arc::new(SheepEntity::new(
        &vanilla_entities::SHEEP,
        next_entity_id(),
        DVec3::new(4.5, 65.0, 6.5),
        Arc::downgrade(world),
    ));
    let sheep_living = sheep.as_living_entity().expect("sheep is living");
    sheep_living.set_health(5.0);
    sheep_living.living_base().set_hurt_time(4);
    sheep_living
        .living_base()
        .replace_active_mob_effects(vec![MobEffectInstance::with_duration(
            vanilla_mob_effects::SPEED,
            200,
            1,
        )]);
    sheep_living.living_base().equipment().lock().set(
        EquipmentSlot::MainHand,
        ItemStack::new(&vanilla_items::DIAMOND_SWORD),
    );
    let player_uuid = uuid::Uuid::from_u128(0x1234_5678);
    sheep_living.set_last_hurt_by_player(player_uuid, 100);

    let prepared = world
        .chunk_map
        .with_chunk_at_status(chunk_pos, ChunkStatus::Full, |chunk| {
            ChunkStorage::prepare_chunk_save(
                chunk,
                ChunkStatus::Full,
                slice::from_ref(&sheep),
                true,
            )
        })
        .flatten()
        .expect("the chunk with the sheep prepares for saving");

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    let dir = temp_dir().join(format!("vecta-region-sheep-{unique}"));
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("encoding pool");
    let regions = RegionManager::new(dir.clone());
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime");

    let loaded = runtime.block_on(async {
        assert!(
            regions
                .save_chunk_data(prepared, &pool)
                .await
                .expect("region write"),
            "the prepared chunk is written to the region file"
        );
        assert!(
            regions
                .acquire_chunk(chunk_pos)
                .await
                .expect("region acquire"),
            "the saved chunk is present in the region index"
        );
        let storage = ChunkStorage::Disk(regions);
        storage
            .load_chunk(
                chunk_pos,
                world.dimension_type.min_y,
                world.dimension_type.height,
                Weak::new(),
                &pool,
            )
            .await
            .expect("region read")
            .expect("the region file holds the chunk")
    });

    let restored = loaded
        .pending_entities
        .iter()
        .find(|entity| entity.entity_type().key == vanilla_entities::SHEEP.key)
        .expect("the region file restores the sheep");
    let restored_living = restored
        .as_living_entity()
        .expect("the restored sheep is a living entity");

    assert_eq!(
        restored_living.get_health(),
        5.0,
        "damaged health survives the region file instead of healing to full"
    );
    assert_eq!(restored_living.living_base().hurt_time(), 4);
    let effect = restored_living
        .mob_effect(vanilla_mob_effects::SPEED)
        .expect("the active effect survives the region file");
    assert_eq!(effect.duration(), 200);
    assert_eq!(effect.amplifier(), 1);
    restored_living.with_equipment_slot(EquipmentSlot::MainHand, &mut |item| {
        assert_eq!(
            item.item().key,
            vanilla_items::DIAMOND_SWORD.key,
            "equipment survives the region file"
        );
    });
    assert_eq!(
        restored_living.last_hurt_by_player_uuid(),
        Some(player_uuid),
        "the combat reference survives the region file as a UUID"
    );

    fs::remove_dir_all(dir).expect("temporary region directory is removed");
}
