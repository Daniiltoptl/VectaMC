//! `/data modify … compute` against live world targets.

use std::sync::Arc;

use vecta_registry::vanilla_blocks;
use vecta_utils::BlockPos;

use super::data_command::{run, source, try_run, with_world};
use super::*;
use crate::scoreboard::ScoreHolder;

/// Seeds a storage key so every test starts from a known compound.
fn seed_storage(server: &Arc<Server>, source: &CommandSource, key: &str) {
    assert!(
        run(
            server,
            source,
            &format!("data modify storage {key} marker set value 1b")
        )
        .0
    );
}

/// `data get storage <key> <path> <scale>` reports `floor(value * scale)`.
fn read_scaled(
    server: &Arc<Server>,
    source: &CommandSource,
    key: &str,
    path: &str,
    scale: f64,
) -> (bool, i32) {
    run(
        server,
        source,
        &format!("data get storage {key} {path} {scale}"),
    )
}

#[test]
fn compute_writes_inline_constants_into_storage() {
    with_world(
        "data_compute_constants",
        |server, _world, player, _packets| {
            let source = source(server, player);
            seed_storage(server, &source, "compute:consts");

            assert_eq!(
                run(
                    server,
                    &source,
                    "data modify storage compute:consts speed set compute default float 1.5"
                ),
                (true, 1)
            );
            // A bare number is `ConstantValue.INLINE_CODEC`; scaling by 2 proves the
            // stored value is 1.5 and not a rounded integer.
            assert_eq!(
                read_scaled(server, &source, "compute:consts", "speed", 2.0),
                (true, 3)
            );

            assert_eq!(
                run(
                    server,
                    &source,
                    "data modify storage compute:consts ticks set compute default integer 7"
                ),
                (true, 1)
            );
            assert_eq!(
                read_scaled(server, &source, "compute:consts", "ticks", 1.0),
                (true, 7)
            );

            // The inline object form goes through the same codec as Java.
            assert_eq!(
                run(
                    server,
                    &source,
                    "data modify storage compute:consts from_int set compute default integer {type:\"minecraft:constant\",value:9}"
                ),
                (true, 1)
            );
            assert_eq!(
                read_scaled(server, &source, "compute:consts", "from_int", 1.0),
                (true, 9)
            );
        },
    );
}

#[test]
fn compute_block_context_uses_the_real_block_state() {
    with_world("data_compute_block", |server, world, player, _packets| {
        let source = source(server, player);
        seed_storage(server, &source, "compute:block");

        let furnace = BlockPos::new(1, 64, 1);
        let smoker = BlockPos::new(2, 64, 2);
        assert!(world.set_block(
            furnace,
            vanilla_blocks::FURNACE.default_state(),
            UpdateFlags::UPDATE_ALL
        ));
        assert!(world.set_block(
            smoker,
            vanilla_blocks::SMOKER.default_state(),
            UpdateFlags::UPDATE_ALL
        ));

        // `cooking/speed_default` is the `block/fast_cooking` conditional, so the
        // same provider yields 1.0 on a furnace and 2.0 on a smoker.
        let furnace_command = format!(
            "data modify storage compute:block furnace_speed set compute block {fx} {fy} {fz} float minecraft:cooking/speed_default",
            fx = furnace.x(),
            fy = furnace.y(),
            fz = furnace.z()
        );
        assert_eq!(run(server, &source, &furnace_command), (true, 1));
        let smoker_command = format!(
            "data modify storage compute:block smoker_speed set compute block {sx} {sy} {sz} float minecraft:cooking/speed_default",
            sx = smoker.x(),
            sy = smoker.y(),
            sz = smoker.z()
        );
        assert_eq!(run(server, &source, &smoker_command), (true, 1));

        assert_eq!(
            read_scaled(server, &source, "compute:block", "furnace_speed", 1.0),
            (true, 1)
        );
        assert_eq!(
            read_scaled(server, &source, "compute:block", "smoker_speed", 1.0),
            (true, 2)
        );
    });
}

#[test]
fn compute_nested_arithmetic_scales_the_result() {
    with_world(
        "data_compute_arithmetic",
        |server, _world, player, _packets| {
            let source = source(server, player);
            seed_storage(server, &source, "compute:arithmetic");

            assert_eq!(
                run(
                    server,
                    &source,
                    "data modify storage compute:arithmetic quotient set compute default float {type:\"minecraft:div\",left:7.0f,right:2.0f}"
                ),
                (true, 1)
            );
            // floor(3.5 * 2) proves the stored value is 3.5.
            assert_eq!(
                read_scaled(server, &source, "compute:arithmetic", "quotient", 2.0),
                (true, 7)
            );
        },
    );
}

#[test]
fn compute_failures_report_errors_and_leave_the_target_unchanged() {
    with_world("data_compute_errors", |server, _world, player, _packets| {
        let source = source(server, player);
        seed_storage(server, &source, "compute:errors");

        // An unknown registry key fails while parsing the argument.
        assert!(!try_run(server, &source, "data modify storage compute:errors missing set compute default integer minecraft:cooking/does_not_exist").0);
        // An inline type the family cannot evaluate is rejected.
        assert!(!try_run(server, &source, "data modify storage compute:errors unknown set compute default integer {type:\"minecraft:environment_attribute\"}").0);
        // Malformed SNBT never reaches the evaluation.
        assert!(
            !try_run(
                server,
                &source,
                "data modify storage compute:errors broken set compute default float {type:"
            )
            .0
        );
        // A block source outside the loaded fixture chunks has no block state.
        assert!(
            !try_run(
                server,
                &source,
                "data modify storage compute:errors unloaded set compute block 900 64 900 integer 1"
            )
            .0
        );

        // The only mutation that ever succeeded is the seeding marker.
        assert_eq!(
            read_scaled(server, &source, "compute:errors", "marker", 1.0),
            (true, 1)
        );
    });
}

#[test]
fn compute_score_provider_reads_the_live_scoreboard() {
    with_world("data_compute_score", |server, world, player, _packets| {
        let source = source(server, player);
        seed_storage(server, &source, "compute:score");

        let scoreboard = server
            .scoreboards
            .get(world.domain())
            .expect("fixture domain scoreboard");
        let objective = scoreboard.add_objective("coins").expect("objective");
        let holder = ScoreHolder::new("compute_target");

        // The score does not exist yet, so the provider falls back to 42. The
        // score-holder identity for a non-player name is the literal name.
        assert_eq!(
            run(
                server,
                &source,
                "data modify storage compute:score coins set compute default integer {type:\"minecraft:score\",target:\"compute_target\",score:\"coins\",fallback:42}"
            ),
            (true, 1)
        );
        assert_eq!(
            read_scaled(server, &source, "compute:score", "coins", 1.0),
            (true, 42)
        );

        // A live score replaces the fallback without restarting the server.
        scoreboard
            .set_score(&holder, &objective, 5)
            .expect("score write");
        assert_eq!(
            run(
                server,
                &source,
                "data modify storage compute:score live set compute default integer {type:\"minecraft:score\",target:\"compute_target\",score:\"coins\"}"
            ),
            (true, 1)
        );
        assert_eq!(
            read_scaled(server, &source, "compute:score", "live", 1.0),
            (true, 5)
        );
    });
}
