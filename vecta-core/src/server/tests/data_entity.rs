//! Entity NBT commands against live world entities.

use super::data_command::{run, source, with_world};
use super::*;
use crate::entity::LivingEntity;
use crate::inventory::equipment::EquipmentSlot;
use crate::scoreboard::ScoreHolder;
use vecta_registry::vanilla_mob_effects;
use vecta_utils::WorldAabb;

pub(super) fn living(entity: &SharedEntity) -> &dyn LivingEntity {
    entity
        .as_living_entity()
        .expect("the fixture is a living entity")
}

pub(super) fn spawn_sheep(world: &Arc<World>, position: DVec3) -> SharedEntity {
    use crate::entity::entities::SheepEntity;

    let sheep = Arc::new(SheepEntity::new(
        &vanilla_entities::SHEEP,
        next_entity_id(),
        position,
        Arc::downgrade(world),
    ));
    world
        .try_add_entity(Arc::clone(&sheep) as SharedEntity)
        .expect("sheep in a loaded chunk");
    sheep
}

#[test]
fn data_entity_reads_and_rewrites_live_entity_state() {
    with_world(
        "data_entity_live_state",
        |server, world, player, _packets| {
            let sheep = spawn_sheep(world, DVec3::new(30.5, 65.0, 30.5));
            let source = source(server, player);

            assert_eq!(
                run(
                    server,
                    &source,
                    "data get entity @e[type=minecraft:sheep,limit=1] Pos"
                ),
                (true, 3),
                "`Pos` is a collection, so `get` reports its size"
            );

            assert_eq!(
                run(
                    server,
                    &source,
                    "data merge entity @e[type=minecraft:sheep,limit=1] {CustomName:'Named',NoGravity:1b}"
                ),
                (true, 1)
            );
            assert_eq!(
                sheep
                    .custom_name()
                    .map(|name| name.to_plain(&DisplayResolutor)),
                Some("Named".to_owned()),
                "the live entity receives the merged name"
            );
            assert!(sheep.is_no_gravity());

            assert_eq!(
                run(
                    server,
                    &source,
                    "data modify entity @e[type=minecraft:sheep,limit=1] Motion set value [1.0d,0.0d,0.0d]"
                ),
                (true, 1)
            );
            assert_eq!(sheep.velocity().x, 1.0);

            assert_eq!(
                run(
                    server,
                    &source,
                    "data remove entity @e[type=minecraft:sheep,limit=1] NoGravity"
                ),
                (true, 1)
            );
            assert!(!sheep.is_no_gravity());
        },
    );
}

#[test]
fn data_entity_resets_omitted_fields_and_keeps_the_uuid() {
    with_world(
        "data_entity_reset_semantics",
        |server, world, player, _packets| {
            let sheep = spawn_sheep(world, DVec3::new(30.5, 65.0, 30.5));
            let uuid = sheep.uuid();
            let source = source(server, player);

            assert_eq!(
                run(
                    server,
                    &source,
                    "data merge entity @e[type=minecraft:sheep,limit=1] {CustomName:'Named',NoGravity:1b,Tags:[\"marked\"]}"
                ),
                (true, 1)
            );
            assert!(sheep.is_no_gravity());

            // `remove` drops a field from the compound that is written back, so
            // vanilla `Entity.load` resets it to the default instead of keeping
            // the previous value.
            assert_eq!(
                run(
                    server,
                    &source,
                    "data remove entity @e[type=minecraft:sheep,limit=1] NoGravity"
                ),
                (true, 1)
            );
            assert!(
                !sheep.is_no_gravity(),
                "an absent NoGravity is reset instead of kept"
            );
            assert_eq!(
                run(
                    server,
                    &source,
                    "data remove entity @e[type=minecraft:sheep,limit=1] CustomName"
                ),
                (true, 1)
            );
            assert!(
                sheep.custom_name().is_none(),
                "an absent CustomName is reset instead of kept"
            );
            assert_eq!(
                run(
                    server,
                    &source,
                    "data remove entity @e[type=minecraft:sheep,limit=1] Tags"
                ),
                (true, 1)
            );
            assert!(sheep.tags().is_empty(), "absent Tags are cleared");

            // Vanilla loads the payload and then restores the original UUID.
            assert_eq!(
                run(
                    server,
                    &source,
                    "data merge entity @e[type=minecraft:sheep,limit=1] {UUID:[I;0,0,0,0]}"
                ),
                (true, 1)
            );
            assert_eq!(
                sheep.uuid(),
                uuid,
                "a payload UUID cannot rename the entity identity"
            );
        },
    );
}

#[test]
fn data_entity_refuses_player_data_and_reports_failure() {
    with_world(
        "data_entity_player_rejection",
        |server, world, player, _packets| {
            let _ = world;
            let source = source(server, player);
            let (success, _) = run(server, &source, "data merge entity @s {CustomName:'\"X\"'}");
            assert!(
                !success,
                "vanilla `EntityDataAccessor.setData` rejects player data"
            );
            assert!(player.custom_name().is_none());
        },
    );
}

#[test]
fn execute_store_entity_writes_through_the_shared_loader() {
    with_world("execute_store_entity", |server, world, player, _packets| {
        let sheep = spawn_sheep(world, DVec3::new(30.5, 65.0, 30.5));
        let source = source(server, player);

        let (success, stored) = run(
            server,
            &source,
            "execute store result entity @e[type=minecraft:sheep,limit=1] PortalCooldown int 1 run data get entity @s Air",
        );
        assert!(success);
        assert!(stored > 0, "the inner `data get` reports the air supply");
        assert_eq!(
            sheep.portal_cooldown(),
            stored,
            "`execute store entity` writes through the same live entity loader"
        );
    });
}

#[test]
fn execute_store_entity_rejects_player_writes_and_multiple_targets() {
    with_world(
        "execute_store_entity_rejections",
        |server, world, player, _| {
            let source = source(server, player);
            let cooldown = player.portal_cooldown();
            let result = run(
                server,
                &source,
                "execute store result entity @s PortalCooldown int 1 run data get entity @s Air",
            );
            assert!(result.0, "a rejected store does not fail the inner command");
            assert_eq!(
                player.portal_cooldown(),
                cooldown,
                "the shared loader rejects player NBT"
            );

            let first = spawn_sheep(world, DVec3::new(30.5, 65.0, 30.5));
            let second = spawn_sheep(world, DVec3::new(31.5, 65.0, 30.5));
            let dispatcher = server.command_dispatcher.read();
            let parse = dispatcher.parse(
                "execute store result entity @e[type=minecraft:sheep] PortalCooldown int 1 run data get entity @s Air",
                source,
            );
            assert!(
                dispatcher.context_chain(parse).is_err(),
                "the command takes a single-entity argument"
            );
            assert_eq!(first.portal_cooldown(), 0);
            assert_eq!(second.portal_cooldown(), 0);
        },
    );
}

#[test]
fn data_entity_decodes_numeric_vectors_and_updates_world_tracking() {
    with_world("data_entity_numeric_vectors", |server, world, player, _| {
        let sheep = spawn_sheep(world, DVec3::new(30.5, 65.0, 30.5));
        let source = source(server, player);
        assert_eq!(
            run(
                server,
                &source,
                "data modify entity @e[type=minecraft:sheep,limit=1] Pos set value [35,66,30]"
            ),
            (true, 1)
        );
        assert_eq!(sheep.position(), DVec3::new(35.0, 66.0, 30.0));
        let old = WorldAabb::new(30.0, 64.0, 30.0, 31.0, 68.0, 31.0);
        let new = WorldAabb::new(34.0, 64.0, 29.0, 36.0, 68.0, 31.0);
        assert!(
            world
                .get_entities_in_aabb_matching(&old, |entity| entity.uuid() == sheep.uuid())
                .is_empty()
        );
        assert_eq!(
            world
                .get_entities_in_aabb_matching(&new, |entity| entity.uuid() == sheep.uuid())
                .len(),
            1
        );
        sheep.base().clear_velocity_sync();
        assert_eq!(
            run(
                server,
                &source,
                "data modify entity @e[type=minecraft:sheep,limit=1] Motion set value [1,0,0]"
            ),
            (true, 1)
        );
        assert_eq!(sheep.velocity(), DVec3::X);
        assert!(sheep.base().needs_velocity_sync());
        assert_eq!(
            run(
                server,
                &source,
                "data modify entity @e[type=minecraft:sheep,limit=1] Rotation set value [45,20]"
            ),
            (true, 1)
        );
        assert_eq!(sheep.rotation(), (45.0, 20.0));
        assert_eq!(
            run(
                server,
                &source,
                "data modify entity @e[type=minecraft:sheep,limit=1] Motion set value [1,2,3,4]"
            ),
            (true, 1)
        );
        assert_eq!(
            sheep.velocity(),
            DVec3::new(1.0, 2.0, 3.0),
            "TagValueInput accepts Util.fixedSize's partial prefix for an overlong list"
        );
        assert_eq!(
            run(
                server,
                &source,
                "data modify entity @e[type=minecraft:sheep,limit=1] Motion set value [1,2]"
            ),
            (true, 1)
        );
        assert_eq!(
            sheep.velocity(),
            DVec3::ZERO,
            "a short list has no partial vector"
        );
        assert_eq!(
            run(
                server,
                &source,
                "data modify entity @e[type=minecraft:sheep,limit=1] Pos set value [I;36,66,30]"
            ),
            (true, 1)
        );
        assert_eq!(sheep.position(), DVec3::new(36.0, 66.0, 30.0));
    });
}

#[test]
fn data_entity_preserves_damage_cooldown_and_uses_numeric_tag_conversions() {
    with_world("data_entity_numeric_fields", |server, world, player, _| {
        let sheep = spawn_sheep(world, DVec3::new(30.5, 65.0, 30.5));
        let living = sheep.as_living_entity().expect("living sheep");
        living.set_invulnerable_time(17);
        let source = source(server, player);
        assert_eq!(
            run(
                server,
                &source,
                "data merge entity @e[type=minecraft:sheep,limit=1] {NoGravity:1,Silent:256,Air:4294967297L,Fire:65537}"
            ),
            (true, 1)
        );
        assert!(sheep.is_no_gravity());
        assert!(!sheep.is_silent(), "boolean conversion uses the low byte");
        assert_eq!(sheep.air_supply(), 1);
        assert_eq!(sheep.fire_freeze_state().remaining_fire_ticks(), 1);
        assert_eq!(
            living.invulnerable_time(),
            17,
            "unrelated data writes retain serialized cooldown"
        );
        assert_eq!(
            run(
                server,
                &source,
                "data modify entity @e[type=minecraft:sheep,limit=1] invulnerable_time set value 7"
            ),
            (true, 1)
        );
        assert_eq!(living.invulnerable_time(), 7);
        assert_eq!(
            run(
                server,
                &source,
                "data remove entity @e[type=minecraft:sheep,limit=1] invulnerable_time"
            ),
            (true, 1)
        );
        assert_eq!(living.invulnerable_time(), 0);
    });
}

#[test]
fn data_merge_writes_living_health_attributes_and_timers() {
    with_world(
        "data_entity_living_health",
        |server, world, player, _packets| {
            let sheep = spawn_sheep(world, DVec3::new(30.5, 65.0, 30.5));
            let source = source(server, player);
            assert_eq!(living(&sheep).get_health(), living(&sheep).get_max_health());

            assert_eq!(
                run(
                    server,
                    &source,
                    "data merge entity @e[type=minecraft:sheep,limit=1] {Health:5.0f,HurtTime:3s,AbsorptionAmount:2.0f}"
                ),
                (true, 1)
            );
            assert_eq!(
                living(&sheep).get_health(),
                5.0,
                "`Health` reaches the live living entity, not just the NBT dump"
            );
            assert_eq!(living(&sheep).living_base().hurt_time(), 3);
            assert_eq!(living(&sheep).get_absorption_amount(), 2.0);

            // `setHealth` clamps against the loaded max-health attribute, so the
            // attribute load has to happen first.
            assert_eq!(
                run(
                    server,
                    &source,
                    "data merge entity @e[type=minecraft:sheep,limit=1] {attributes:[{id:\"minecraft:max_health\",base:20.0d}]}"
                ),
                (true, 1)
            );
            assert_eq!(living(&sheep).get_max_health(), 20.0);
            assert_eq!(
                run(
                    server,
                    &source,
                    "data merge entity @e[type=minecraft:sheep,limit=1] {Health:50.0f}"
                ),
                (true, 1)
            );
            assert_eq!(
                living(&sheep).get_health(),
                20.0,
                "health is clamped to the max-health attribute that was just loaded"
            );

            assert_eq!(
                run(
                    server,
                    &source,
                    "data remove entity @e[type=minecraft:sheep,limit=1] Health"
                ),
                (true, 1)
            );
            assert_eq!(
                living(&sheep).get_health(),
                20.0,
                "an omitted Health resets to max health"
            );
            assert_eq!(
                run(
                    server,
                    &source,
                    "data remove entity @e[type=minecraft:sheep,limit=1] HurtTime"
                ),
                (true, 1)
            );
            assert_eq!(living(&sheep).living_base().hurt_time(), 0);
        },
    );
}

#[test]
fn data_merge_writes_active_effects_and_equipment() {
    with_world(
        "data_entity_living_effects",
        |server, world, player, _packets| {
            let sheep = spawn_sheep(world, DVec3::new(30.5, 65.0, 30.5));
            let source = source(server, player);

            assert_eq!(
                run(
                    server,
                    &source,
                    "data merge entity @e[type=minecraft:sheep,limit=1] {active_effects:[{id:\"minecraft:speed\",duration:200,amplifier:1b,show_particles:0b}]}"
                ),
                (true, 1)
            );
            let effect = living(&sheep)
                .mob_effect(vanilla_mob_effects::SPEED)
                .expect("the loaded effect is active");
            assert_eq!(effect.duration(), 200);
            assert_eq!(effect.amplifier(), 1);
            assert!(
                !effect.is_visible(),
                "`show_particles:0b` survives the effect codec"
            );

            assert_eq!(
                run(
                    server,
                    &source,
                    "data remove entity @e[type=minecraft:sheep,limit=1] active_effects"
                ),
                (true, 1)
            );
            assert!(
                living(&sheep)
                    .mob_effect(vanilla_mob_effects::SPEED)
                    .is_none(),
                "an omitted active_effects list clears every effect"
            );

            assert_eq!(
                run(
                    server,
                    &source,
                    "data merge entity @e[type=minecraft:sheep,limit=1] {equipment:{mainhand:{id:\"minecraft:diamond_sword\",count:1}}}"
                ),
                (true, 1)
            );
            living(&sheep).with_equipment_slot(EquipmentSlot::MainHand, &mut |item| {
                assert_eq!(
                    item.item().key,
                    vanilla_items::DIAMOND_SWORD.key,
                    "the equipment compound reaches the live slot"
                );
            });

            assert_eq!(
                run(
                    server,
                    &source,
                    "data remove entity @e[type=minecraft:sheep,limit=1] equipment"
                ),
                (true, 1)
            );
            living(&sheep).with_equipment_slot(EquipmentSlot::MainHand, &mut |item| {
                assert!(
                    item.is_empty(),
                    "an omitted equipment compound clears every slot"
                );
            });
        },
    );
}

#[test]
fn data_merge_team_uses_the_domain_scoreboard_and_the_loaded_uuid() {
    with_world("data_entity_team", |server, world, player, _packets| {
        let sheep = spawn_sheep(world, DVec3::new(30.5, 65.0, 30.5));
        let source = source(server, player);
        let domain = world.domain().to_owned();
        let scoreboard = server
            .scoreboards
            .get(&domain)
            .expect("the fixture domain has a scoreboard");
        scoreboard.add_team("red").expect("team is created");
        let holder = |uuid: uuid::Uuid| ScoreHolder::new(uuid.to_string());
        let entity_uuid = sheep.uuid();

        assert_eq!(
            run(
                server,
                &source,
                "data merge entity @e[type=minecraft:sheep,limit=1] {Team:\"red\"}"
            ),
            (true, 1)
        );
        assert_eq!(
            scoreboard.holder_team_name(&holder(entity_uuid)),
            Some("red".to_owned()),
            "`Team` adds the entity to the domain scoreboard"
        );

        // Vanilla logs a warning and keeps the old membership instead of failing.
        assert_eq!(
            run(
                server,
                &source,
                "data merge entity @e[type=minecraft:sheep,limit=1] {Team:\"blue\"}"
            ),
            (true, 1)
        );
        assert_eq!(
            scoreboard.holder_team_name(&holder(entity_uuid)),
            Some("red".to_owned()),
            "an unknown team does not change membership"
        );

        // Vanilla adds the membership under the payload UUID and only then
        // restores the original identity.
        let payload_uuid = uuid::Uuid::from_u128(0xfeed_face);
        let payload = payload_uuid.to_int_array();
        assert_eq!(
            run(
                server,
                &source,
                &format!(
                    "data merge entity @e[type=minecraft:sheep,limit=1] {{UUID:[I;{},{},{},{}],Team:\"red\"}}",
                    payload[0], payload[1], payload[2], payload[3]
                )
            ),
            (true, 1)
        );
        assert_eq!(
            sheep.uuid(),
            entity_uuid,
            "a payload UUID cannot rename the live entity"
        );
        assert_eq!(
            scoreboard.holder_team_name(&holder(payload_uuid)),
            Some("red".to_owned()),
            "the membership follows the UUID the payload carried"
        );
    });
}
