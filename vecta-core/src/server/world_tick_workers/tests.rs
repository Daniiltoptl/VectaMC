use crossbeam::channel::{Receiver, TryRecvError};
use futures::{FutureExt, executor::block_on};

use super::*;
use crate::test_support::test_domain;

#[test]
fn persistent_workers_tick_every_world_across_boundaries() {
    let worlds = test_domain("workers", &["primary", "derived"]);
    let workers = WorldTickWorkers::spawn(worlds.values()).expect("workers");
    for tick in 1..=2 {
        worlds.advance_domain_game_times();
        let timings = block_on(workers.tick_all(tick, true)).expect("world ticks");
        assert_eq!(timings.len(), 2);
        for world in worlds.values() {
            assert_eq!(world.game_time(), tick as i64);
        }
    }
}

// Keep the real request/response channels under test control, so dispatch order
// can be checked without sleeps or depending on the OS thread scheduler.
fn controlled_workers<'a>(
    worlds: impl IntoIterator<Item = &'a Arc<World>>,
) -> (WorldTickWorkers, Vec<Receiver<WorldTickRequest>>) {
    let (workers, receivers) = worlds
        .into_iter()
        .map(|world| {
            let (sender, receiver) = channel::bounded(1);
            (
                WorldTickWorker {
                    world_key: Arc::from(world.key.to_string()),
                    requests: Some(sender),
                    thread: None,
                },
                receiver,
            )
        })
        .unzip();
    (WorldTickWorkers { workers }, receivers)
}

#[test]
fn next_world_is_dispatched_only_after_the_previous_tick_finishes() {
    let worlds = test_domain("ordered", &["primary", "derived"]);
    let (workers, receivers) = controlled_workers(worlds.values());
    for (tick, runs_normally) in [(1, true), (2, false), (3, true)] {
        if runs_normally {
            worlds.advance_domain_game_times();
        }
        let mut pending = Box::pin(workers.tick_all(tick, runs_normally));
        for (index, world) in worlds.values().enumerate() {
            assert!(pending.as_mut().now_or_never().is_none());
            let request = receivers[index].try_recv().expect("current world request");
            assert_eq!(
                (request.tick_count, request.runs_normally),
                (tick, runs_normally)
            );
            for receiver in &receivers[index + 1..] {
                assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
            }
            let timings = world.tick_game(request.tick_count, request.runs_normally);
            request
                .response
                .send(timings)
                .expect("return actual world timings");
        }
        let timings = block_on(pending).expect("all worlds finish");
        assert_eq!(timings.len(), worlds.len());
    }
}

#[test]
fn a_missing_response_stops_before_ticking_later_worlds() {
    let worlds = test_domain("failed_tick", &["primary", "derived"]);
    let (workers, receivers) = controlled_workers(worlds.values());
    let mut pending = Box::pin(workers.tick_all(1, true));
    assert!(pending.as_mut().now_or_never().is_none());
    drop(receivers[0].try_recv().expect("first world request"));
    assert!(matches!(
        block_on(pending),
        Err(WorldTickWorkerError::MissingResponse { .. })
    ));
    assert!(matches!(receivers[1].try_recv(), Err(TryRecvError::Empty)));
}
