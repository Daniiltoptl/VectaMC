//! Loaded living state exercised through real command and world paths.

use super::data_command::{run, source, with_world};
use super::data_entity::{living, spawn_sheep};
use super::*;
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::blocks::properties::{BedPart, BlockStateProperties, Direction};
use vecta_registry::{vanilla_attributes, vanilla_mob_effects};

#[test]
fn living_load_accepts_numeric_attribute_codecs_and_partial_modifiers() {
    with_world(
        "living_load_numeric_attributes",
        |server, world, player, _| {
            let sheep = spawn_sheep(world, DVec3::new(30.5, 65.0, 30.5));
            let source = source(server, player);
            assert_eq!(
                run(
                    server,
                    &source,
                    "data merge entity @e[type=minecraft:sheep,limit=1] {Health:50,attributes:[{id:'minecraft:max_health',base:40,modifiers:[{id:'vecta:valid',amount:2,operation:'add_value'},{id:'vecta:invalid',amount:'bad',operation:'add_value'}]}]}"
                ),
                (true, 1)
            );
            assert_eq!(living(&sheep).get_max_health(), 42.0);
            assert_eq!(living(&sheep).get_health(), 42.0);
            assert_eq!(
                living(&sheep)
                    .attributes()
                    .lock()
                    .get_base_value(vanilla_attributes::MAX_HEALTH),
                Some(40.0)
            );
        },
    );
}

#[test]
fn living_load_retains_valid_effects_and_decodes_codec_numbers() {
    with_world("living_load_partial_effects", |server, world, player, _| {
        let sheep = spawn_sheep(world, DVec3::new(30.5, 65.0, 30.5));
        let source = source(server, player);
        assert_eq!(
            run(
                server,
                &source,
                "data merge entity @e[type=minecraft:sheep,limit=1] {active_effects:[{id:'minecraft:speed',amplifier:2,duration:-1.9d,ambient:0.5d,show_particles:256},3,{id:'vecta:missing'},{id:'minecraft:haste',duration:'bad'}]}"
            ),
            (true, 1)
        );
        let speed = living(&sheep)
            .mob_effect(vanilla_mob_effects::SPEED)
            .expect("valid speed survives a partial list");
        assert_eq!(speed.amplifier(), 2);
        assert_eq!(speed.duration(), -1);
        assert!(speed.is_ambient());
        assert!(speed.is_visible());
        assert!(speed.show_icon());
        let haste = living(&sheep)
            .mob_effect(vanilla_mob_effects::HASTE)
            .expect("partial optional field retains its default");
        assert_eq!(haste.duration(), 0);
        assert!(
            living(&sheep)
                .living_base()
                .drain_dirty_mob_effects()
                .is_empty(),
            "NBT loading does not run gameplay effect packet hooks"
        );
    });
}

#[test]
fn living_load_narrows_short_timers_and_keeps_memory_time_without_a_reference() {
    with_world("living_load_timers", |server, world, player, _| {
        let sheep = spawn_sheep(world, DVec3::new(30.5, 65.0, 30.5));
        let source = source(server, player);
        assert_eq!(
            run(
                server,
                &source,
                "data merge entity @e[type=minecraft:sheep,limit=1] {HurtTime:65537,DeathTime:65538,last_hurt_by_player_memory_time:40}"
            ),
            (true, 1)
        );
        assert_eq!(living(&sheep).living_base().hurt_time(), 1);
        assert_eq!(living(&sheep).living_base().death_time(), 2);
        assert!(living(&sheep).last_hurt_by_player_uuid().is_none());
        assert_eq!(living(&sheep).last_hurt_by_player_memory_time(), 40);
    });
}

#[test]
fn living_load_sleep_repositions_without_starting_a_new_sleep_interaction() {
    with_world("living_load_sleep", |server, world, player, _| {
        let sheep = spawn_sheep(world, DVec3::new(30.5, 65.0, 30.5));
        let foot = BlockPos::new(31, 65, 30);
        let head = foot.east();
        for pos in [foot, head] {
            world.set_block(
                pos.below(),
                vanilla_blocks::STONE.default_state(),
                UpdateFlags::UPDATE_ALL,
            );
        }
        let state = vanilla_blocks::RED_BED
            .default_state()
            .set_value(&BlockStateProperties::HORIZONTAL_FACING, Direction::East)
            .set_value(&BlockStateProperties::BED_PART, BedPart::Foot);
        let flags = UpdateFlags::UPDATE_CLIENTS | UpdateFlags::UPDATE_KNOWN_SHAPE;
        assert!(world.set_block(foot, state, flags));
        assert!(world.set_block(
            head,
            state.set_value(&BlockStateProperties::BED_PART, BedPart::Head),
            flags
        ));
        sheep.set_first_tick(false);
        let source = source(server, player);
        assert_eq!(
            run(
                server,
                &source,
                "data merge entity @e[type=minecraft:sheep,limit=1] {sleeping_pos:[32.8d,65.9d,30.1d,999.0d],Motion:[0.25d,0.0d,0.0d]}"
            ),
            (true, 1)
        );
        assert_eq!(living(&sheep).sleeping_pos(), Some(head));
        assert_eq!(sheep.velocity().x, 0.25);
        assert!(
            !world
                .get_block_state(head)
                .get_value(&BlockStateProperties::OCCUPIED)
        );
        assert_eq!(sheep.position().x, 32.5);
        assert_eq!(sheep.position().z, 30.5);
        assert_eq!(
            run(
                server,
                &source,
                "data merge entity @e[type=minecraft:sheep,limit=1] {sleeping_pos:[32,65,30,'bad']}"
            ),
            (true, 1)
        );
        assert!(living(&sheep).sleeping_pos().is_none());
    });
}

#[test]
fn loaded_combat_references_require_living_targets_and_expire_on_tick() {
    with_world(
        "living_load_combat_reference",
        |server, world, player, _| {
            let victim = spawn_sheep(world, DVec3::new(30.5, 65.0, 30.5));
            let attacker = spawn_sheep(world, DVec3::new(31.5, 65.0, 30.5));
            let item = world
                .spawn_item(
                    DVec3::new(30.5, 65.0, 31.5),
                    ItemStack::new(&vanilla_items::STONE),
                )
                .expect("item");
            let source = source(server, player);
            for (target, expired) in [(item.uuid(), false), (attacker.uuid(), true)] {
                let uuid = target.to_int_array();
                let command = format!(
                    "data merge entity {} {{last_hurt_by_mob:[{},{},{},{},77],ticks_since_last_hurt_by_mob:101}}",
                    victim.uuid(),
                    uuid[0],
                    uuid[1],
                    uuid[2],
                    uuid[3]
                );
                assert_eq!(run(server, &source, &command), (true, 1));
                assert_eq!(
                    living(&victim).last_hurt_by_mob_uuid(),
                    Some(target),
                    "UUID codecs retain a valid partial prefix"
                );
                if expired {
                    living(&victim).base_tick_living_entity();
                    assert!(living(&victim).last_hurt_by_mob_uuid().is_none());
                } else {
                    assert!(living(&victim).last_hurt_by_mob().is_none());
                }
            }
        },
    );
}
