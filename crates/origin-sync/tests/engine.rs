//! Scheduling behaviour, tested by moving a fake clock rather than by sleeping.

use async_trait::async_trait;
use origin_domain::testing::FakeClock;
use origin_domain::{
    AccountId, AppError, Clock, ConnectorId, ErrorKind, Health, Result, SyncOutcome,
};
use origin_events::{EventBus, PlatformEvent};
use origin_storage::MemoryStorage;
use origin_sync::{
    Backoff, SyncContext, SyncEngine, SyncPolicy, SyncReport, SyncResult, SyncSource, SyncTarget,
};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use time::Duration;
use time::macros::datetime;
use tokio::sync::{Barrier, Notify};

const NOW: time::OffsetDateTime = datetime!(2026-08-23 10:00 UTC);

/// A source that returns whatever the test queues, and records what it was given.
#[derive(Debug, Default)]
struct ScriptedSource {
    responses: Mutex<Vec<Result<SyncResult>>>,
    calls: AtomicU32,
    seen_etags: Mutex<Vec<Option<String>>>,
}

impl ScriptedSource {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn queue(self: &Arc<Self>, result: Result<SyncResult>) -> &Arc<Self> {
        self.responses.lock().unwrap().push(result);
        self
    }

    fn calls(&self) -> u32 {
        self.calls.load(Ordering::SeqCst)
    }

    fn seen_etags(&self) -> Vec<Option<String>> {
        self.seen_etags.lock().unwrap().clone()
    }
}

#[async_trait]
impl SyncSource for ScriptedSource {
    async fn sync(&self, context: &SyncContext) -> Result<SyncResult> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.seen_etags
            .lock()
            .unwrap()
            .push(context.etag().map(str::to_owned));

        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            return Ok(SyncResult::NotModified);
        }
        responses.remove(0)
    }
}

#[derive(Debug)]
struct BlockingSource {
    calls: AtomicU32,
    entered: Barrier,
    release: Notify,
}

impl BlockingSource {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicU32::new(0),
            entered: Barrier::new(2),
            release: Notify::new(),
        })
    }
}

#[async_trait]
impl SyncSource for BlockingSource {
    async fn sync(&self, _context: &SyncContext) -> Result<SyncResult> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.entered.wait().await;
        self.release.notified().await;
        Ok(SyncResult::NotModified)
    }
}

struct Harness {
    engine: SyncEngine,
    clock: Arc<FakeClock>,
    events: EventBus,
}

fn harness() -> Harness {
    let clock = Arc::new(FakeClock::new(NOW));
    let events = EventBus::new();
    let engine = SyncEngine::new(
        Arc::new(MemoryStorage::new()),
        clock.clone(),
        events.clone(),
    );

    Harness {
        engine,
        clock,
        events,
    }
}

fn target(name: &str) -> SyncTarget {
    SyncTarget::new(ConnectorId::new("demo"), AccountId::new("acc-1"), name)
}

/// No jitter, so due times are exact.
fn policy(interval: Duration) -> SyncPolicy {
    SyncPolicy::every(interval).with_backoff(Backoff {
        base: Duration::seconds(30),
        max: Duration::minutes(30),
        multiplier: 2,
        jitter: 0.0,
    })
}

#[tokio::test]
async fn a_new_target_is_due_immediately() {
    let harness = harness();
    let source = ScriptedSource::new();
    let notifications = target("notifications");

    harness.engine.register(
        notifications.clone(),
        policy(Duration::minutes(5)),
        source.clone(),
    );

    harness.engine.run_due(harness.clock.now()).await;

    assert_eq!(source.calls(), 1);
}

#[tokio::test]
async fn a_target_waits_for_its_interval() {
    let harness = harness();
    let source = ScriptedSource::new();
    harness.engine.register(
        target("notifications"),
        policy(Duration::minutes(5)),
        source.clone(),
    );

    harness.engine.run_due(harness.clock.now()).await;
    assert_eq!(source.calls(), 1);

    harness.clock.advance(Duration::minutes(4));
    harness.engine.run_due(harness.clock.now()).await;
    assert_eq!(source.calls(), 1, "it is not due yet");

    harness.clock.advance(Duration::minutes(2));
    harness.engine.run_due(harness.clock.now()).await;
    assert_eq!(source.calls(), 2);
}

#[tokio::test]
async fn targets_with_different_cadences_run_independently() {
    let harness = harness();
    let fast = ScriptedSource::new();
    let slow = ScriptedSource::new();

    harness.engine.register(
        target("notifications"),
        policy(Duration::minutes(1)),
        fast.clone(),
    );
    harness.engine.register(
        target("projects"),
        policy(Duration::minutes(10)),
        slow.clone(),
    );

    for _ in 0..5 {
        harness.engine.run_due(harness.clock.now()).await;
        harness
            .clock
            .advance(Duration::minutes(1) + Duration::seconds(1));
    }

    assert_eq!(fast.calls(), 5);
    assert_eq!(
        slow.calls(),
        1,
        "the slow target must not follow the fast one"
    );
}

#[tokio::test]
async fn failures_back_off_exponentially_and_recover() {
    let harness = harness();
    let source = ScriptedSource::new();
    source
        .queue(Err(AppError::Network("timeout".into())))
        .queue(Err(AppError::Network("timeout".into())))
        .queue(Ok(SyncResult::Updated(SyncReport::changed(3))));

    let notifications = target("notifications");
    harness.engine.register(
        notifications.clone(),
        policy(Duration::minutes(5)),
        source.clone(),
    );

    // First failure → 30 s, not the 5-minute interval.
    harness.engine.run_due(harness.clock.now()).await;
    assert_eq!(
        harness.engine.due_at(&notifications).await.unwrap(),
        NOW + Duration::seconds(30)
    );

    harness.clock.advance(Duration::seconds(31));
    harness.engine.run_due(harness.clock.now()).await;
    assert_eq!(source.calls(), 2);

    // Second failure → 60 s.
    let after_second = harness.clock.now();
    assert_eq!(
        harness.engine.due_at(&notifications).await.unwrap(),
        after_second + Duration::seconds(60)
    );

    harness.clock.advance(Duration::seconds(61));
    harness.engine.run_due(harness.clock.now()).await;

    let state = harness.engine.state(&notifications).await.unwrap();
    assert_eq!(state.failure_streak, 0, "a success resets the streak");
    assert_eq!(
        harness.engine.due_at(&notifications).await.unwrap(),
        harness.clock.now() + Duration::minutes(5),
        "and the normal cadence returns"
    );
}

#[tokio::test]
async fn being_offline_retries_soon_instead_of_backing_off_for_half_an_hour() {
    let harness = harness();
    let source = ScriptedSource::new();
    for _ in 0..6 {
        source.queue(Err(AppError::Offline("no route to host".into())));
    }

    let notifications = target("notifications");
    let policy = policy(Duration::minutes(5)).with_offline_retry(Duration::seconds(20));
    harness
        .engine
        .register(notifications.clone(), policy, source.clone());

    for _ in 0..6 {
        harness.engine.run_due(harness.clock.now()).await;
        harness.clock.advance(Duration::seconds(21));
    }

    assert_eq!(
        source.calls(),
        6,
        "connectivity usually returns in one step; exponential backoff would leave the \
         app stale long after the network came back"
    );

    let state = harness.engine.state(&notifications).await.unwrap();
    assert!(matches!(
        state.last_outcome,
        Some(SyncOutcome::Failed {
            kind: ErrorKind::Offline,
            ..
        })
    ));
}

#[tokio::test]
async fn validators_are_handed_back_on_the_next_run() {
    let harness = harness();
    let source = ScriptedSource::new();
    source
        .queue(Ok(SyncResult::Updated(
            SyncReport::changed(2).with_etag("etag-1"),
        )))
        .queue(Ok(SyncResult::NotModified));

    harness.engine.register(
        target("notifications"),
        policy(Duration::minutes(5)),
        source.clone(),
    );

    harness.engine.run_due(harness.clock.now()).await;
    harness.clock.advance(Duration::minutes(6));
    harness.engine.run_due(harness.clock.now()).await;

    assert_eq!(source.seen_etags(), vec![None, Some("etag-1".to_owned())]);
}

#[tokio::test]
async fn a_response_without_a_validator_does_not_clear_the_stored_one() {
    let harness = harness();
    let source = ScriptedSource::new();
    source
        .queue(Ok(SyncResult::Updated(
            SyncReport::changed(1).with_etag("etag-1"),
        )))
        // A service that returns data but omits the ETag this time.
        .queue(Ok(SyncResult::Updated(SyncReport::changed(1))));

    let notifications = target("notifications");
    harness.engine.register(
        notifications.clone(),
        policy(Duration::minutes(5)),
        source.clone(),
    );

    harness.engine.run_due(harness.clock.now()).await;
    harness.clock.advance(Duration::minutes(6));
    harness.engine.run_due(harness.clock.now()).await;

    assert_eq!(
        harness
            .engine
            .state(&notifications)
            .await
            .unwrap()
            .etag
            .as_deref(),
        Some("etag-1"),
        "dropping the validator would turn every later sync into a full refetch"
    );
}

#[tokio::test]
async fn a_manual_sync_reports_its_outcome_and_publishes_an_event() {
    let harness = harness();
    let mut stream = harness.events.subscribe::<PlatformEvent>().unwrap();
    let source = ScriptedSource::new();
    source.queue(Ok(SyncResult::Updated(SyncReport::changed(7))));

    let notifications = target("notifications");
    harness
        .engine
        .register(notifications.clone(), policy(Duration::minutes(5)), source);

    let outcome = harness.engine.sync_now(&notifications).await.unwrap();
    assert_eq!(outcome, SyncOutcome::Updated);

    match stream.recv().await.unwrap() {
        PlatformEvent::SyncCompleted(event) => {
            assert_eq!(event.changed, 7);
            assert_eq!(event.connector, ConnectorId::new("demo"));
        }
        other => panic!("expected SyncCompleted, got {other:?}"),
    }
}

#[tokio::test]
async fn a_failed_sync_publishes_when_it_will_be_retried() {
    let harness = harness();
    let mut stream = harness.events.subscribe::<PlatformEvent>().unwrap();
    let source = ScriptedSource::new();
    source.queue(Err(AppError::RateLimited {
        message: "secondary limit".into(),
        retry_after_seconds: Some(60),
    }));

    let notifications = target("notifications");
    harness
        .engine
        .register(notifications.clone(), policy(Duration::minutes(5)), source);

    harness.engine.sync_now(&notifications).await.unwrap_err();

    match stream.recv().await.unwrap() {
        PlatformEvent::SyncFailed(event) => {
            assert_eq!(event.kind, ErrorKind::RateLimited);
            assert_eq!(event.retry_at, Some(NOW + Duration::seconds(30)));
        }
        other => panic!("expected SyncFailed, got {other:?}"),
    }
}

#[tokio::test]
async fn syncing_an_unknown_target_is_a_validation_error() {
    let harness = harness();

    let error = harness.engine.sync_now(&target("nope")).await.unwrap_err();

    assert_eq!(error.kind(), ErrorKind::Validation);
}

#[tokio::test]
async fn health_reports_the_worst_registered_target() {
    let harness = harness();
    let healthy = ScriptedSource::new();
    let broken = ScriptedSource::new();
    for _ in 0..3 {
        broken.queue(Err(AppError::Network("timeout".into())));
    }

    harness.engine.register(
        target("notifications"),
        policy(Duration::minutes(5)),
        healthy,
    );
    harness
        .engine
        .register(target("projects"), policy(Duration::minutes(5)), broken);

    for _ in 0..3 {
        harness.engine.run_due(harness.clock.now()).await;
        harness.clock.advance(Duration::minutes(6));
    }

    assert_eq!(harness.engine.health().await, Health::Critical);
}

#[tokio::test]
async fn state_survives_unregistering_and_registering_again() {
    let harness = harness();
    let notifications = target("notifications");

    let first = ScriptedSource::new();
    first.queue(Ok(SyncResult::Updated(
        SyncReport::changed(1).with_etag("etag-1"),
    )));
    harness
        .engine
        .register(notifications.clone(), policy(Duration::minutes(5)), first);
    harness.engine.run_due(harness.clock.now()).await;

    harness.engine.unregister(&notifications);
    assert!(harness.engine.targets().is_empty());

    let second = ScriptedSource::new();
    harness.engine.register(
        notifications.clone(),
        policy(Duration::minutes(5)),
        second.clone(),
    );

    harness.clock.advance(Duration::minutes(6));
    harness.engine.run_due(harness.clock.now()).await;

    assert_eq!(
        second.seen_etags(),
        vec![Some("etag-1".to_owned())],
        "restarting the app must not throw away validators"
    );
}

#[tokio::test]
async fn a_repeated_trigger_is_throttled_but_an_explicit_refresh_is_not() {
    let harness = harness();
    let source = ScriptedSource::new();
    let notifications = target("notifications");

    let policy = policy(Duration::minutes(5)).with_min_interval(Duration::seconds(30));
    harness
        .engine
        .register(notifications.clone(), policy, source.clone());

    // First trigger runs; the next three are within the throttle window.
    assert!(
        harness
            .engine
            .sync_if_due(&notifications)
            .await
            .unwrap()
            .is_some()
    );
    for _ in 0..3 {
        assert!(
            harness
                .engine
                .sync_if_due(&notifications)
                .await
                .unwrap()
                .is_none(),
            "alt-tabbing twenty times must not mean twenty syncs"
        );
    }
    assert_eq!(source.calls(), 1);

    // The user pressing Refresh asked explicitly and gets a run.
    harness.engine.sync_now(&notifications).await.unwrap();
    assert_eq!(source.calls(), 2);

    harness.clock.advance(Duration::seconds(31));
    assert!(
        harness
            .engine
            .sync_if_due(&notifications)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(source.calls(), 3);
}

#[tokio::test]
async fn concurrent_triggers_recheck_the_throttle_after_waiting_for_the_same_target() {
    let harness = harness();
    let source = BlockingSource::new();
    let notifications = target("notifications");
    harness.engine.register(
        notifications.clone(),
        policy(Duration::minutes(5)).with_min_interval(Duration::seconds(30)),
        source.clone(),
    );

    let first = {
        let engine = harness.engine.clone();
        let target = notifications.clone();
        tokio::spawn(async move { engine.sync_if_due(&target).await })
    };
    source.entered.wait().await;

    let second = {
        let engine = harness.engine.clone();
        let target = notifications.clone();
        tokio::spawn(async move { engine.sync_if_due(&target).await })
    };
    source.release.notify_one();

    assert!(first.await.unwrap().unwrap().is_some());
    assert!(second.await.unwrap().unwrap().is_none());
    assert_eq!(source.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn one_slow_target_does_not_block_other_due_targets() {
    let harness = harness();
    let slow = BlockingSource::new();
    let fast = ScriptedSource::new();
    harness
        .engine
        .register(target("a-slow"), policy(Duration::minutes(5)), slow.clone());
    harness
        .engine
        .register(target("z-fast"), policy(Duration::minutes(5)), fast.clone());

    let run = {
        let engine = harness.engine.clone();
        tokio::spawn(async move { engine.run_due(NOW).await })
    };
    slow.entered.wait().await;
    tokio::task::yield_now().await;

    assert_eq!(fast.calls(), 1);
    slow.release.notify_one();
    run.await.unwrap();
}

#[tokio::test]
async fn a_scheduler_run_does_not_re_sync_a_target_a_concurrent_manual_sync_already_covered() {
    let harness = harness();
    let source = BlockingSource::new();
    let notifications = target("notifications");
    harness.engine.register(
        notifications.clone(),
        policy(Duration::minutes(5)),
        source.clone(),
    );

    // A manual sync starts first and holds the target's single-flight lock for the
    // whole exchange below.
    let manual = {
        let engine = harness.engine.clone();
        let target = notifications.clone();
        tokio::spawn(async move { engine.sync_now(&target).await })
    };
    source.entered.wait().await;

    // The scheduler sees the target as still due (nothing has completed yet) and spawns
    // a run for it, which blocks behind the manual sync's lock.
    let scheduled = {
        let engine = harness.engine.clone();
        tokio::spawn(async move { engine.run_due(NOW).await })
    };
    tokio::task::yield_now().await;

    // Releasing the manual sync lets it finish and free the lock — the scheduler's
    // queued run must then recheck the schedule instead of blindly firing again.
    source.release.notify_one();

    manual.await.unwrap().unwrap();
    let results = scheduled.await.unwrap();

    assert_eq!(
        source.calls.load(Ordering::SeqCst),
        1,
        "the target must not have synced twice"
    );
    assert!(
        results.is_empty(),
        "the scheduler should have found nothing left to do, got {results:?}"
    );
}

#[tokio::test]
async fn the_scheduler_stops_when_asked() {
    let harness = harness();
    let source = ScriptedSource::new();
    harness.engine.register(
        target("notifications"),
        policy(Duration::minutes(5)),
        source.clone(),
    );

    let stop = tokio_util::sync::CancellationToken::new();
    let engine = harness.engine.clone();
    let scheduler = tokio::spawn({
        let stop = stop.clone();
        async move { engine.run(stop).await }
    });

    // The first tick fires immediately, so the due target runs.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(source.calls(), 1);

    stop.cancel();
    tokio::time::timeout(std::time::Duration::from_secs(2), scheduler)
        .await
        .expect("the scheduler must end promptly when cancelled")
        .expect("scheduler task");
}
