use std::env::temp_dir;
use std::path::PathBuf;

use tokio::{fs, runtime::Builder};
use uuid::Uuid;
use vecta_utils::random::Random as _;

use super::*;

fn directory() -> PathBuf {
    temp_dir().join(format!("vecta-random-sequences-{}", Uuid::new_v4()))
}

#[tokio::test]
async fn saved_stream_resumes_after_the_last_persisted_draw() {
    let path = directory();
    let storage = SavedDataManager::new(Some(&path));
    let streams = RandomSequences::load(1_234, &storage)
        .await
        .expect("new streams");
    let key = Identifier::vanilla_static("chests/simple_dungeon");
    streams.with_sequence(&key, |rng| {
        rng.next_i64();
        rng.next_f32();
    });
    assert!(streams.save(&storage).await.expect("save advanced streams"));
    assert!(
        !streams
            .save(&storage)
            .await
            .expect("unchanged streams stay clean")
    );
    let expected = streams.with_sequence(&key, |rng| {
        (rng.next_i64(), rng.next_i32_bounded(17), rng.next_f32())
    });

    let restored = RandomSequences::load(1_234, &storage)
        .await
        .expect("load saved streams");
    let actual = restored.with_sequence(&key, |rng| {
        (rng.next_i64(), rng.next_i32_bounded(17), rng.next_f32())
    });
    assert_eq!(actual, expected);
    fs::remove_dir_all(path)
        .await
        .expect("remove owned temporary storage");
}

#[test]
fn a_draw_after_the_save_snapshot_remains_dirty() {
    let streams = RandomSequences::new(42);
    let key = Identifier::vanilla_static("chests/simple_dungeon");
    streams.with_sequence(&key, RandomSource::next_i64);
    let snapshot = streams.pending_save().expect("snapshot").expect("dirty");
    streams.with_sequence(&key, RandomSource::next_i64);
    streams.mark_saved(snapshot.revision);
    assert!(
        streams
            .pending_save()
            .expect("snapshot after concurrent draw")
            .is_some()
    );
}

#[tokio::test]
async fn a_failed_write_does_not_acknowledge_the_stream_state() {
    let path = directory();
    fs::write(&path, b"not a directory")
        .await
        .expect("create owned obstruction");
    let storage = SavedDataManager::new(Some(&path));
    let streams = RandomSequences::new(42);
    streams.with_sequence(&Identifier::from_vecta("failure"), RandomSource::next_i64);
    assert!(streams.save(&storage).await.is_err());
    assert!(streams.pending_save().expect("snapshot").is_some());
    fs::remove_file(path)
        .await
        .expect("remove owned obstruction");
}

#[tokio::test]
async fn seed_defaults_and_signed_seed_words_survive_persistence() {
    let path = directory();
    let storage = SavedDataManager::new(Some(&path));
    let data = PersistentRandomSequences {
        salt: -7,
        include_world_seed: false,
        include_sequence_id: false,
        sequences: BTreeMap::new(),
    };
    storage
        .save(RANDOM_SEQUENCES, &data)
        .await
        .expect("seed defaults");
    let streams = RandomSequences::load(1_234, &storage)
        .await
        .expect("load defaults");
    let first_key = Identifier::from_vecta("first");
    let second_key = Identifier::from_vecta("second");
    let mut expected = Xoroshiro::from_seed((-7_i64) as u64);
    let first = expected.next_i64();
    assert_eq!(
        streams.with_sequence(&first_key, RandomSource::next_i64),
        first
    );
    assert_eq!(
        streams.with_sequence(&second_key, RandomSource::next_i64),
        first
    );
    streams
        .save(&storage)
        .await
        .expect("save signed seed words");
    let restored = RandomSequences::load(9_999, &storage)
        .await
        .expect("reload with another world seed");
    assert_eq!(
        restored.with_sequence(&first_key, RandomSource::next_i64),
        expected.next_i64()
    );
    fs::remove_dir_all(path)
        .await
        .expect("remove owned temporary storage");
}

#[tokio::test]
async fn invalid_sequence_keys_are_reported_instead_of_resetting_saved_loot() {
    let path = directory();
    let storage = SavedDataManager::new(Some(&path));
    let mut data = PersistentRandomSequences::default();
    data.sequences.insert(
        "minecraft:BAD KEY".to_owned(),
        PersistentSequence { source: [1, 2] },
    );
    storage
        .save(RANDOM_SEQUENCES, &data)
        .await
        .expect("invalid saved entry");
    assert!(RandomSequences::load(42, &storage).await.is_err());
    fs::remove_dir_all(path)
        .await
        .expect("remove owned temporary storage");
}

#[test]
fn hosted_worlds_share_the_server_primary_seed_and_one_advancing_stream() {
    use crate::bootstrap::init_globals;
    use crate::config::ResolvedDomainConfig;
    use crate::test_support::fresh_test_world_in_domain;

    init_globals();
    let primary = fresh_test_world_in_domain("main", "overworld");
    let other = fresh_test_world_in_domain("other", "overworld");
    primary.world.level_data.write().data_mut().seed = 1_234;
    other.world.level_data.write().data_mut().seed = 9_999;
    let domains = [
        ResolvedDomainConfig {
            name: "main".to_owned(),
            default_world: primary.world.key.clone(),
            worlds: vec![primary.world.key.clone()],
        },
        ResolvedDomainConfig {
            name: "other".to_owned(),
            default_world: other.world.key.clone(),
            worlds: vec![other.world.key.clone()],
        },
    ];
    let mut worlds = WorldMap::new("main".to_owned(), &domains, &[]);
    worlds.insert(primary.world.key.clone(), Arc::clone(&primary.world));
    worlds.insert(other.world.key.clone(), Arc::clone(&other.world));
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime");
    let _owner = runtime
        .block_on(RandomSequences::load_and_bind(&worlds))
        .expect("server streams");
    let expected = RandomSequences::new(1_234);
    let key = Identifier::vanilla_static("chests/simple_dungeon");
    for world in [&primary.world, &other.world, &primary.world] {
        assert_eq!(
            world.random().with_sequence(&key, RandomSource::next_i64),
            expected.with_sequence(&key, RandomSource::next_i64),
        );
    }
}
