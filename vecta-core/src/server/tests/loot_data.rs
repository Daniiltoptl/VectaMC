//! Live shared-domain loot reads through the same state as commands.

use super::data_command::{run, source, with_world};
use super::data_entity::spawn_sheep;
use super::*;
use crate::entity::entity_loot_ref;
use crate::scoreboard::ScoreHolder;
use crate::test_support::test_domain;
use text_components::TextComponent;
use vecta_registry::loot_table::{
    LootCondition, LootContext, LootContextEntity, LootDataReader, LootFunction, LootScoreHolder,
    NumberProvider, NumberProviderRange, ScoreboardTarget, loot_context_params as params,
};
use vecta_utils::{
    Identifier,
    nbt::parse_nbt_path,
    random::{Random as _, RandomSource, legacy_random::LegacyRandom},
};

fn with_domains(test: impl FnOnce(&Arc<Server>, &Arc<World>, &Arc<World>, &Arc<World>)) {
    let alpha = test_domain("loot_alpha", &["primary", "derived"]);
    let beta = test_domain("loot_beta", &["primary"]);
    let domains: Vec<_> = [&alpha, &beta]
        .iter()
        .map(|map| ResolvedDomainConfig {
            name: map.default_domain().to_owned(),
            default_world: map
                .server_default_world()
                .expect("default world")
                .key
                .clone(),
            worlds: map.keys().cloned().collect(),
        })
        .collect();
    let loaded: Vec<_> = alpha.values().chain(beta.values()).cloned().collect();
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    runtime.block_on(async {
        let server = test_server_with_worlds(
            "loot_alpha".to_owned(),
            &domains,
            &loaded,
            PermissionSubjectIndex::new(),
        )
        .await
        .expect("server");
        let primary = server.worlds.default_world("loot_alpha").expect("alpha");
        let derived = server
            .worlds
            .get(&Identifier::new_static("loot_alpha", "derived"))
            .expect("derived");
        let other = server.worlds.default_world("loot_beta").expect("beta");
        test(&server, primary, derived, other);
    });
}

#[test]
fn loot_reads_share_live_domain_state_and_keep_other_domains_isolated() {
    with_domains(|server, primary, derived, other| {
        let alpha = server
            .scoreboards
            .get("loot_alpha")
            .expect("alpha scoreboard");
        let beta = server
            .scoreboards
            .get("loot_beta")
            .expect("beta scoreboard");
        let alpha_objective = alpha.add_objective("points").expect("objective");
        let beta_objective = beta.add_objective("points").expect("objective");
        let holder = ScoreHolder::new("Holder");
        alpha
            .set_score(&holder, &alpha_objective, 7)
            .expect("alpha score");
        beta.set_score(&holder, &beta_objective, 3)
            .expect("beta score");
        let provider = NumberProvider::Score {
            target: ScoreboardTarget::Fixed("Holder"),
            score: "points",
        };
        let mut rng = RandomSource::Legacy(LegacyRandom::from_seed(42));
        let mut context = LootContext::new(&mut rng).with_data(derived.as_ref());
        assert_eq!(provider.get_int_with_context(&mut context), 7);
        assert_eq!(
            primary.score(LootScoreHolder::Name("Holder"), "points"),
            Some(7)
        );
        assert_eq!(
            other.score(LootScoreHolder::Name("Holder"), "points"),
            Some(3)
        );
        let precise_score = i32::MAX - 1;
        alpha
            .set_score(&holder, &alpha_objective, precise_score)
            .expect("changed score");
        assert_eq!(
            provider.get_int_with_context(&mut context),
            precise_score,
            "integer score must not round through f32"
        );
        assert_eq!(
            other.score(LootScoreHolder::Name("Holder"), "points"),
            Some(3)
        );
        let missing = NumberProvider::Score {
            target: ScoreboardTarget::Fixed("Absent"),
            score: "points",
        };
        assert_eq!(missing.get_int_with_context(&mut context), 0);
        assert!(
            alpha
                .score(&ScoreHolder::new("Absent"), &alpha_objective)
                .is_none()
        );
        let mut expected = RandomSource::Legacy(LegacyRandom::from_seed(42));
        assert_eq!(
            context.rng.next_i32(),
            expected.next_i32(),
            "reads consume no loot randomness"
        );
    });
}

#[test]
fn storage_readers_share_live_updates_within_one_domain() {
    with_domains(|server, primary, derived, other| {
        let storage_key = Identifier::from_vecta("loot_live");
        let mut value = NbtCompound::new();
        value.insert("value", 11);
        server
            .command_storage
            .get("loot_alpha")
            .expect("storage")
            .set(storage_key.clone(), value);
        let path = parse_nbt_path("value").expect("path");
        assert_eq!(primary.storage_int(&storage_key, &path), Some(11));
        assert_eq!(derived.storage_int(&storage_key, &path), Some(11));
        assert_eq!(other.storage_int(&storage_key, &path), None);
        let mut changed = NbtCompound::new();
        changed.insert("value", 23);
        server
            .command_storage
            .get("loot_alpha")
            .expect("storage")
            .set(storage_key.clone(), changed);
        assert_eq!(derived.storage_int(&storage_key, &path), Some(23));
    });
}
#[test]
fn storage_number_consumers_follow_live_data_commands_and_boxed_number_casts() {
    with_world("loot_storage_reader", |server, world, player, _| {
        let source = source(server, player);
        assert_eq!(
            run(
                server,
                &source,
                "data merge storage vecta:loot {value:11,wide:4294967297L,fraction:-1.9d,text:'wrong',many:[1,2]}"
            ),
            (true, 1)
        );
        let provider = |path| NumberProvider::Storage {
            storage: Identifier::from_vecta("loot"),
            path: parse_nbt_path(path).expect("path"),
        };
        let mut rng = RandomSource::Legacy(LegacyRandom::from_seed(7));
        let mut context = LootContext::new(&mut rng).with_data(world.as_ref());
        let mut stack = ItemStack::new(&vanilla_items::STONE);
        LootFunction::SetCount {
            count: provider("value"),
            add: false,
        }
        .apply(&mut stack, &mut context);
        assert_eq!(stack.count(), 11);
        assert_eq!(provider("wide").get_int_with_context(&mut context), 1);
        assert_eq!(provider("fraction").get_int_with_context(&mut context), -1);
        assert_eq!(
            provider("fraction").get_float(&mut context).to_bits(),
            (-1.9_f32).to_bits()
        );
        assert_eq!(provider("text").get_int_with_context(&mut context), 0);
        assert_eq!(provider("many[]").get_int_with_context(&mut context), 0);
        assert_eq!(
            run(
                server,
                &source,
                "data modify storage vecta:loot value set value 29"
            ),
            (true, 1)
        );
        LootFunction::SetCount {
            count: provider("value"),
            add: false,
        }
        .apply(&mut stack, &mut context);
        assert_eq!(stack.count(), 29);
        assert_eq!(
            run(server, &source, "data remove storage vecta:loot value"),
            (true, 1)
        );
        assert_eq!(provider("value").get_int_with_context(&mut context), 0);
    });
}

#[test]
fn score_entity_targets_use_profile_names_and_uuids_instead_of_display_names() {
    static SCORES: [(&str, NumberProviderRange); 1] =
        [("points", NumberProviderRange::exact(11.0))];
    static MISSING: [(&str, NumberProviderRange); 1] =
        [("absent", NumberProviderRange::exact(0.0))];
    with_world("loot_score_identity", |server, world, player, _| {
        let matches_score = |entity, ctx: &mut LootContext<'_>| {
            LootCondition::EntityScores {
                entity,
                scores: &SCORES,
            }
            .test(ctx)
        };
        let scoreboard = server.scoreboards.get(world.domain()).expect("scoreboard");
        let objective = scoreboard.add_objective("points").expect("objective");
        scoreboard
            .set_score(&ScoreHolder::new(player.gameprofile.name()), &objective, 11)
            .expect("player score");
        let sheep = spawn_sheep(world, DVec3::new(30.5, 65.0, 30.5));
        sheep.set_custom_name(Some(TextComponent::plain(
            player.gameprofile.name().to_owned(),
        )));
        scoreboard
            .set_score(&ScoreHolder::new(sheep.uuid().to_string()), &objective, 37)
            .expect("mob score");
        let mut rng = RandomSource::Legacy(LegacyRandom::from_seed(8));
        let mut context = LootContext::new(&mut rng)
            .with_data(world.as_ref())
            .with_parameter(params::THIS_ENTITY, entity_loot_ref(player.as_ref(), None));
        let this = NumberProvider::Score {
            target: ScoreboardTarget::This,
            score: "points",
        };
        assert_eq!(this.get_int_with_context(&mut context), 11);
        assert!(matches_score(LootContextEntity::This, &mut context));
        assert!(!matches_score(LootContextEntity::Target, &mut context));
        context.params.set(
            params::THIS_ENTITY,
            Some(entity_loot_ref(sheep.as_ref(), None)),
        );
        context.params.set(
            params::TARGET_ENTITY,
            Some(entity_loot_ref(player.as_ref(), None)),
        );
        assert_eq!(this.get_int_with_context(&mut context), 37);
        assert!(!matches_score(LootContextEntity::This, &mut context));
        let target = NumberProvider::Score {
            target: ScoreboardTarget::Target,
            score: "points",
        };
        assert_eq!(target.get_int_with_context(&mut context), 11);
        assert!(matches_score(LootContextEntity::Target, &mut context));
        assert!(
            !LootCondition::EntityScores {
                entity: LootContextEntity::Target,
                scores: &MISSING
            }
            .test(&mut context)
        );
    });
}
