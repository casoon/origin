use origin_domain::testing::FakeClock;
use origin_domain::{Clock, JobStatus};
use origin_events::{EventBus, PlatformEvent};
use origin_jobs::Jobs;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use time::macros::datetime;

fn jobs() -> (Jobs, EventBus) {
    let events = EventBus::new();
    let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(datetime!(2026-08-23 10:00 UTC)));
    (Jobs::new(events.clone(), clock), events)
}

#[tokio::test(flavor = "current_thread")]
async fn a_spawned_job_is_immediately_visible_and_cancellable() {
    let (jobs, _) = jobs();
    let id = jobs.spawn("waiting", |ctx| async move {
        ctx.cancelled().await;
        Ok(())
    });

    assert_eq!(jobs.get(&id).await.unwrap().status, JobStatus::Queued);
    jobs.cancel(&id).await.unwrap();
}

/// Wait until `predicate` holds, or fail. Jobs run on their own task, so a test cannot
/// assume they finished the moment `spawn` returned.
async fn eventually<F, Fut>(what: &str, mut predicate: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    for _ in 0..200 {
        if predicate().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("timed out waiting for: {what}");
}

#[tokio::test]
async fn a_successful_job_ends_as_succeeded() {
    let (jobs, _) = jobs();

    let id = jobs.spawn("export", |_ctx| async { Ok(()) });

    let probe = jobs.clone();
    let probe_id = id.clone();
    eventually("job to finish", || {
        let jobs = probe.clone();
        let id = probe_id.clone();
        async move {
            jobs.get(&id)
                .await
                .is_some_and(|job| job.status.is_terminal())
        }
    })
    .await;

    let job = jobs.get(&id).await.unwrap();
    assert_eq!(job.status, JobStatus::Succeeded);
    assert_eq!(job.kind, "export");
    assert!(job.finished_at.is_some());
    assert!(!job.cancelable, "a finished job cannot be cancelled");
}

#[tokio::test]
async fn a_failing_job_records_the_error() {
    let (jobs, _) = jobs();

    let id = jobs.spawn("export", |_ctx| async {
        Err(origin_domain::AppError::storage("disk full"))
    });

    let probe = jobs.clone();
    let probe_id = id.clone();
    eventually("job to fail", || {
        let jobs = probe.clone();
        let id = probe_id.clone();
        async move {
            jobs.get(&id)
                .await
                .is_some_and(|job| job.status.is_terminal())
        }
    })
    .await;

    let job = jobs.get(&id).await.unwrap();
    assert_eq!(job.status, JobStatus::Failed);
    assert!(job.error.unwrap().contains("disk full"));
}

#[tokio::test]
async fn a_panicking_job_is_recorded_as_failed_not_lost() {
    let (jobs, _) = jobs();

    let id = jobs.spawn("broken", |_ctx| async {
        panic!("something went very wrong");
    });

    let probe = jobs.clone();
    let probe_id = id.clone();
    eventually("panicking job to be recorded", || {
        let jobs = probe.clone();
        let id = probe_id.clone();
        async move {
            jobs.get(&id)
                .await
                .is_some_and(|job| job.status.is_terminal())
        }
    })
    .await;

    let job = jobs.get(&id).await.unwrap();
    assert_eq!(
        job.status,
        JobStatus::Failed,
        "a job stuck on 'running' forever is worse than one reporting an error"
    );
    assert!(job.error.unwrap().contains("panicked"));
}

#[tokio::test]
async fn cancellation_reaches_the_running_job() {
    let (jobs, _) = jobs();
    let iterations = Arc::new(AtomicU32::new(0));
    let counter = iterations.clone();

    let id = jobs.spawn("loop", move |ctx| async move {
        while !ctx.is_cancelled() {
            counter.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        Ok(())
    });

    let probe = jobs.clone();
    let probe_id = id.clone();
    eventually("job to make progress", || {
        let counter = iterations.clone();
        async move { counter.load(Ordering::SeqCst) > 0 }
    })
    .await;

    jobs.cancel(&id).await.unwrap();

    eventually("job to stop", || {
        let jobs = probe.clone();
        let id = probe_id.clone();
        async move {
            jobs.get(&id)
                .await
                .is_some_and(|job| job.status.is_terminal())
        }
    })
    .await;

    assert_eq!(jobs.get(&id).await.unwrap().status, JobStatus::Cancelled);
}

#[tokio::test]
async fn cancelling_an_unknown_job_is_a_validation_error() {
    let (jobs, _) = jobs();

    let error = jobs
        .cancel(&origin_domain::JobId::new("nope"))
        .await
        .unwrap_err();

    assert_eq!(error.kind(), origin_domain::ErrorKind::Validation);
}

#[tokio::test]
async fn progress_is_throttled_so_the_bus_cannot_be_flooded() {
    let (jobs, events) = jobs();
    let mut stream = events.subscribe::<PlatformEvent>().unwrap();

    // 1000 steps at 0.1 % each — a naive implementation publishes 1000 events and
    // pushes every subscriber into lag.
    jobs.spawn("import", |ctx| async move {
        for step in 1..=1000u64 {
            ctx.progress(step, Some(1000)).await;
        }
        Ok(())
    });

    let mut progress_events = 0;
    let mut finished = false;
    while !finished {
        match tokio::time::timeout(Duration::from_secs(5), stream.recv()).await {
            Ok(Ok(PlatformEvent::JobProgress(_))) => progress_events += 1,
            Ok(Ok(PlatformEvent::JobFinished(_))) => finished = true,
            Ok(Ok(_)) => {}
            Ok(Err(error)) => panic!("subscriber fell behind: {error}"),
            Err(_) => panic!("no finished event within the timeout"),
        }
    }

    assert!(
        progress_events <= 110,
        "expected roughly one event per percent, got {progress_events}"
    );
    assert!(
        progress_events > 50,
        "progress was reported too rarely: {progress_events}"
    );
}

#[tokio::test]
async fn a_job_without_a_known_total_still_reports_progress() {
    let (jobs, events) = jobs();
    let mut stream = events.subscribe::<PlatformEvent>().unwrap();

    let id = jobs.spawn("scan", |ctx| async move {
        ctx.progress(1, None).await;
        ctx.progress(2, None).await;
        Ok(())
    });

    let mut saw_progress = false;
    loop {
        match tokio::time::timeout(Duration::from_secs(5), stream.recv()).await {
            Ok(Ok(PlatformEvent::JobProgress(event))) if event.job == id => {
                saw_progress = true;
                assert_eq!(event.total, None);
            }
            Ok(Ok(PlatformEvent::JobFinished(_))) => break,
            Ok(Ok(_)) => {}
            _ => panic!("no finished event"),
        }
    }

    assert!(
        saw_progress,
        "an indeterminate job must still report that it is alive"
    );
}

#[tokio::test]
async fn running_jobs_are_listed_separately_from_finished_ones() {
    let (jobs, _) = jobs();

    jobs.spawn("quick", |_ctx| async { Ok(()) });
    let slow = jobs.spawn("slow", |ctx| async move {
        ctx.cancelled().await;
        Ok(())
    });

    let probe = jobs.clone();
    eventually("the quick job to finish", || {
        let jobs = probe.clone();
        async move { jobs.list().await.iter().any(|job| job.status.is_terminal()) }
    })
    .await;

    let running = jobs.running().await;
    assert_eq!(running.len(), 1);
    assert_eq!(running[0].kind, "slow");
    assert_eq!(jobs.list().await.len(), 2);

    jobs.cancel(&slow).await.unwrap();
}

#[tokio::test]
async fn spawn_exclusive_refuses_a_second_job_of_the_same_kind() {
    let (jobs, _) = jobs();

    let _first = jobs
        .spawn_exclusive("crawl", |ctx| async move {
            ctx.cancelled().await;
            Ok(())
        })
        .unwrap();

    let error = jobs
        .spawn_exclusive("crawl", |_ctx| async { Ok(()) })
        .unwrap_err();
    assert_eq!(error.kind(), origin_domain::ErrorKind::Validation);

    // A different kind is unaffected by the exclusivity of "crawl".
    jobs.spawn_exclusive("export", |_ctx| async { Ok(()) })
        .unwrap();
}

#[tokio::test]
async fn spawn_exclusive_allows_a_new_job_once_the_first_finished() {
    let (jobs, _) = jobs();

    let first = jobs
        .spawn_exclusive("crawl", |_ctx| async { Ok(()) })
        .unwrap();

    let probe = jobs.clone();
    eventually("the first job to finish", || {
        let jobs = probe.clone();
        let id = first.clone();
        async move {
            jobs.get(&id)
                .await
                .is_some_and(|job| job.status.is_terminal())
        }
    })
    .await;

    jobs.spawn_exclusive("crawl", |_ctx| async { Ok(()) })
        .expect("a finished job does not hold the exclusivity lock");
}

#[tokio::test]
async fn spawn_awaitable_returns_the_bodys_value() {
    let (jobs, _) = jobs();

    let (_id, result) = jobs.spawn_awaitable("render", |_ctx| async { Ok(42u32) });

    assert_eq!(result.wait().await.unwrap(), 42);
}

#[tokio::test]
async fn spawn_awaitable_surfaces_the_bodys_error() {
    let (jobs, _) = jobs();

    let (_id, result) = jobs.spawn_awaitable("render", |_ctx: origin_jobs::JobContext| async {
        Err::<u32, _>(origin_domain::AppError::storage("disk full"))
    });

    let error = result.wait().await.unwrap_err();
    assert!(error.to_string().contains("disk full"));
}

#[tokio::test]
async fn spawn_exclusive_awaitable_combines_both() {
    let (jobs, _) = jobs();

    let (_id, first) = jobs
        .spawn_exclusive_awaitable("crawl", |ctx| async move {
            ctx.cancelled().await;
            Ok::<_, origin_domain::AppError>("first".to_string())
        })
        .unwrap();

    let conflict = jobs.spawn_exclusive_awaitable::<_, _, ()>("crawl", |_ctx| async { Ok(()) });
    assert!(conflict.is_err(), "a second 'crawl' must be refused");

    jobs.cancel(first.id()).await.unwrap();
    assert_eq!(first.wait().await.unwrap(), "first");

    // Now that it finished, the kind is free again.
    let (_id, second) = jobs
        .spawn_exclusive_awaitable("crawl", |_ctx| async {
            Ok::<_, origin_domain::AppError>("second".to_string())
        })
        .unwrap();
    assert_eq!(second.wait().await.unwrap(), "second");
}
