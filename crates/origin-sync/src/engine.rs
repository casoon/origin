use crate::source::{SyncContext, SyncResult, SyncSource};
use crate::state_store::SyncStateStore;
use crate::{SyncPolicy, SyncTarget, health_of};
use origin_core::{
    AccountId, AppError, Clock, ConnectorId, ErrorKind, Health, Result, SyncId, SyncOutcome,
    SyncState,
};
use origin_events::{EventBus, PlatformEvent, SyncCompleted, SyncFailed};
use origin_storage::Storage;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use time::{Duration, OffsetDateTime};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// How often the background loop looks for due targets.
///
/// Independent of any policy interval: it only has to be fine-grained enough that a
/// target does not drift noticeably past its due time.
const TICK: std::time::Duration = std::time::Duration::from_secs(5);

#[derive(Debug)]
struct Registration {
    policy: SyncPolicy,
    source: Arc<dyn SyncSource>,
    /// Held for the duration of a run, so one target never syncs twice at once.
    running: Arc<Mutex<()>>,
    cancel: CancellationToken,
}

/// Decides when each registered target runs.
#[derive(Debug, Clone)]
pub struct SyncEngine {
    targets: Arc<RwLock<BTreeMap<SyncTarget, Registration>>>,
    state: SyncStateStore,
    clock: Arc<dyn Clock>,
    events: EventBus,
    /// Seed for jitter. Deterministic on purpose: with a fake clock, tests reproduce.
    seed: Arc<AtomicU64>,
}

impl SyncEngine {
    pub fn new(storage: Arc<dyn Storage>, clock: Arc<dyn Clock>, events: EventBus) -> Self {
        let seed = clock.now().unix_timestamp_nanos() as u64 | 1;

        Self {
            targets: Arc::new(RwLock::new(BTreeMap::new())),
            state: SyncStateStore::new(storage, clock.clone()),
            clock,
            events,
            seed: Arc::new(AtomicU64::new(seed)),
        }
    }

    /// Register a target. Registering the same target again replaces it.
    ///
    /// Synchronous so that a module can register from `ApplicationModule::register`,
    /// which runs during startup and has no runtime to await on.
    pub fn register(&self, target: SyncTarget, policy: SyncPolicy, source: Arc<dyn SyncSource>) {
        tracing::debug!(%target, interval = ?policy.interval, "sync target registered");

        self.write().insert(
            target,
            Registration {
                policy,
                source,
                running: Arc::new(Mutex::new(())),
                cancel: CancellationToken::new(),
            },
        );
    }

    /// Stop tracking a target. Any run in flight is cancelled.
    pub fn unregister(&self, target: &SyncTarget) {
        if let Some(registration) = self.write().remove(target) {
            registration.cancel.cancel();
        }
    }

    pub fn targets(&self) -> Vec<SyncTarget> {
        self.read().keys().cloned().collect()
    }

    /// The policy a target was registered with.
    pub fn policy(&self, target: &SyncTarget) -> Option<SyncPolicy> {
        self.read()
            .get(target)
            .map(|registration| registration.policy)
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, BTreeMap<SyncTarget, Registration>> {
        self.targets
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, BTreeMap<SyncTarget, Registration>> {
        self.targets
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub async fn state(&self, target: &SyncTarget) -> Result<SyncState> {
        self.state.load(target).await
    }

    /// When this target may next run.
    ///
    /// Healthy targets follow their interval; failing ones follow the backoff; an
    /// offline machine gets the flat offline retry instead of an exponential one.
    pub async fn due_at(&self, target: &SyncTarget) -> Result<OffsetDateTime> {
        let policy = self
            .read()
            .get(target)
            .map(|registration| registration.policy)
            .ok_or_else(|| AppError::validation(format!("unknown sync target {target}")))?;

        let state = self.state.load(target).await?;
        Ok(self.due_at_for(&state, &policy))
    }

    fn due_at_for(&self, state: &SyncState, policy: &SyncPolicy) -> OffsetDateTime {
        let Some(last_attempt) = state.last_attempt else {
            // Never ran: due immediately.
            return OffsetDateTime::UNIX_EPOCH;
        };

        let delay = match (state.failure_streak, &state.last_outcome) {
            (0, _) => policy.interval,
            (
                _,
                Some(SyncOutcome::Failed {
                    kind: ErrorKind::Offline,
                    ..
                }),
            ) => policy.offline_retry,
            (failures, _) => policy.backoff.delay_for(failures, self.next_random()),
        };

        // No `min_interval` floor here: that is a throttle for *triggered* syncs
        // (see `sync_if_due`). Applying it to the scheduler would silently override an
        // explicitly configured `offline_retry`.
        last_attempt + delay
    }

    /// Sync every target that is due at `now`.
    ///
    /// Separate from the background loop so scheduling can be tested by moving a fake
    /// clock instead of by sleeping.
    pub async fn run_due(&self, now: OffsetDateTime) -> Vec<(SyncTarget, Result<SyncOutcome>)> {
        let candidates: Vec<(SyncTarget, SyncPolicy)> = self
            .read()
            .iter()
            .map(|(target, registration)| (target.clone(), registration.policy))
            .collect();

        let mut results = Vec::new();
        let mut runs = Vec::new();
        for (target, policy) in candidates {
            let state = match self.state.load(&target).await {
                Ok(state) => state,
                Err(error) => {
                    results.push((target, Err(error)));
                    continue;
                }
            };

            if now < self.due_at_for(&state, &policy) {
                continue;
            }

            let engine = self.clone();
            let run_target = target.clone();
            runs.push((
                target,
                tokio::spawn(async move { engine.sync_now(&run_target).await }),
            ));
        }

        for (target, run) in runs {
            let outcome = match run.await {
                Ok(outcome) => outcome,
                Err(error) => Err(AppError::internal(format!(
                    "sync task for {target} failed: {error}"
                ))),
            };
            results.push((target, outcome));
        }

        results
    }

    /// Sync unless the target ran very recently.
    ///
    /// This is the entry point for triggers that fire on their own — window focus,
    /// network coming back, a view being opened. Without the throttle, alt-tabbing
    /// twenty times means twenty syncs.
    ///
    /// Returns `Ok(None)` when the run was skipped. A user pressing *Refresh* should
    /// go through [`SyncEngine::sync_now`] instead: they asked explicitly.
    pub async fn sync_if_due(&self, target: &SyncTarget) -> Result<Option<SyncOutcome>> {
        let (policy, source, running, cancel) = {
            let targets = self.read();
            let registration = targets
                .get(target)
                .ok_or_else(|| AppError::validation(format!("unknown sync target {target}")))?;
            (
                registration.policy,
                registration.source.clone(),
                registration.running.clone(),
                registration.cancel.clone(),
            )
        };

        let _guard = running.lock().await;
        let state = self.state.load(target).await?;
        if let Some(last_attempt) = state.last_attempt
            && self.clock.now() < last_attempt + policy.min_interval
        {
            tracing::debug!(%target, "sync skipped: ran too recently");
            return Ok(None);
        }

        self.sync_with(target, policy, source, cancel, state)
            .await
            .map(Some)
    }

    /// Sync one target immediately, whatever the throttle says.
    ///
    /// Single-flight: a second caller waits for the run in flight instead of starting
    /// a parallel one. Two concurrent syncs of the same target would race on the
    /// validators and could store an older result over a newer one.
    pub async fn sync_now(&self, target: &SyncTarget) -> Result<SyncOutcome> {
        let (policy, source, running, cancel) = {
            let targets = self.read();
            let registration = targets
                .get(target)
                .ok_or_else(|| AppError::validation(format!("unknown sync target {target}")))?;
            (
                registration.policy,
                registration.source.clone(),
                registration.running.clone(),
                registration.cancel.clone(),
            )
        };

        let _guard = running.lock().await;
        let state = self.state.load(target).await?;
        self.sync_with(target, policy, source, cancel, state).await
    }

    async fn sync_with(
        &self,
        target: &SyncTarget,
        policy: SyncPolicy,
        source: Arc<dyn SyncSource>,
        cancel: CancellationToken,
        state: SyncState,
    ) -> Result<SyncOutcome> {
        let sync_id = SyncId::generate();
        let context = SyncContext::new(sync_id.clone(), target.clone(), state.clone(), cancel);

        let span = tracing::info_span!(
            "sync",
            sync_id = sync_id.as_str(),
            connector = target.connector.as_str(),
            account_id = target.account.as_str(),
            target = target.name.as_str(),
        );
        let _entered = span.enter();

        let result = source.sync(&context).await;
        let now = self.clock.now();
        let mut state = state;

        match result {
            Ok(SyncResult::Updated(report)) => {
                state.record(now, SyncOutcome::Updated);
                // Validators are only replaced when the service sent new ones; a
                // response without an ETag must not clear the one we still hold.
                if report.etag.is_some() {
                    state.etag = report.etag.clone();
                }
                if report.last_modified.is_some() {
                    state.last_modified = report.last_modified.clone();
                }
                self.state.save(target, &state).await?;

                tracing::debug!(changed = report.changed, "sync updated");
                self.publish_completed(target, &sync_id, report.changed, now);
                Ok(SyncOutcome::Updated)
            }

            Ok(SyncResult::NotModified) => {
                state.record(now, SyncOutcome::NotModified);
                self.state.save(target, &state).await?;

                tracing::debug!("sync reported no change");
                self.publish_completed(target, &sync_id, 0, now);
                Ok(SyncOutcome::NotModified)
            }

            Err(error) => {
                let outcome = SyncOutcome::Failed {
                    kind: error.kind(),
                    message: error.to_string(),
                };
                state.record(now, outcome.clone());
                self.state.save(target, &state).await?;

                let retry_at = Some(self.due_at_for(&state, &policy));
                tracing::warn!(kind = ?error.kind(), %error, ?retry_at, "sync failed");

                let _ = self.events.publish(PlatformEvent::SyncFailed(SyncFailed {
                    sync: sync_id,
                    connector: target.connector.clone(),
                    account: target.account.clone(),
                    kind: error.kind(),
                    message: error.to_string(),
                    retry_at,
                }));

                Err(error)
            }
        }
    }

    /// Health across all registered targets — the worst state wins.
    pub async fn health(&self) -> Health {
        let now = self.clock.now();
        let targets: Vec<(SyncTarget, SyncPolicy)> = self
            .read()
            .iter()
            .map(|(target, registration)| (target.clone(), registration.policy))
            .collect();

        let mut states = Vec::new();
        for (target, policy) in targets {
            let state = self.state.load(&target).await.unwrap_or_default();
            states.push(health_of(&state, &policy, now));
        }

        Health::aggregate(states)
    }

    /// Health of everything belonging to one account.
    pub async fn health_of_account(&self, connector: &ConnectorId, account: &AccountId) -> Health {
        let now = self.clock.now();
        let targets: Vec<(SyncTarget, SyncPolicy)> = self
            .read()
            .iter()
            .filter(|(target, _)| &target.connector == connector && &target.account == account)
            .map(|(target, registration)| (target.clone(), registration.policy))
            .collect();

        let mut states = Vec::new();
        for (target, policy) in targets {
            let state = self.state.load(&target).await.unwrap_or_default();
            states.push(health_of(&state, &policy, now));
        }

        Health::aggregate(states)
    }

    /// Run the scheduler until `stop` is cancelled.
    ///
    /// Returns a future rather than spawning a task: which executor runs it, and on
    /// which thread, is the host's decision. A platform crate that called
    /// `tokio::spawn` itself would panic wherever no runtime is entered — which is
    /// exactly what a Tauri `setup` hook is.
    ///
    /// A thin wrapper around [`SyncEngine::run_due`]; all the logic worth testing is
    /// in there, not in this loop.
    pub async fn run(&self, stop: CancellationToken) {
        tracing::debug!("sync scheduler started");
        let mut ticker = tokio::time::interval(TICK);

        loop {
            tokio::select! {
                _ = stop.cancelled() => break,
                _ = ticker.tick() => {
                    self.run_due(self.clock.now()).await;
                }
            }
        }

        tracing::debug!("sync scheduler stopped");
    }

    fn publish_completed(
        &self,
        target: &SyncTarget,
        sync_id: &SyncId,
        changed: u64,
        at: OffsetDateTime,
    ) {
        let _ = self
            .events
            .publish(PlatformEvent::SyncCompleted(SyncCompleted {
                sync: sync_id.clone(),
                connector: target.connector.clone(),
                account: target.account.clone(),
                changed,
                at,
            }));
    }

    /// A value in `0.0..1.0` for jitter.
    fn next_random(&self) -> f64 {
        let previous = self
            .seed
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |seed| {
                Some(
                    seed.wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1_442_695_040_888_963_407),
                )
            });

        let value = previous
            .unwrap_or(0)
            .wrapping_mul(6_364_136_223_846_793_005);
        f64::from((value >> 40) as u32) / f64::from(1u32 << 24)
    }
}

/// Convenience for the common "every interval" registration.
impl SyncEngine {
    pub fn register_every(
        &self,
        target: SyncTarget,
        interval: Duration,
        source: Arc<dyn SyncSource>,
    ) {
        self.register(target, SyncPolicy::every(interval), source);
    }
}
