use crate::context::JobContext;
use origin_domain::{AppError, Clock, Job, JobId, JobStatus, Progress, Result};
use origin_events::{EventBus, JobFinished, JobProgress, JobStarted, PlatformEvent};
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, RwLock};
use tokio_util::sync::CancellationToken;

/// How many finished jobs to keep for the UI before dropping the oldest.
///
/// Without a cap, a long-running application accumulates job records forever.
const MAX_FINISHED: usize = 50;

/// Publish a progress event only once the completion changed by this much.
///
/// A job reporting all ten thousand of its steps would otherwise fill the event
/// channel and push slow subscribers into lag — losing them the *finished* event they
/// actually need.
const PROGRESS_STEP: f64 = 0.01;

#[derive(Debug)]
struct Entry {
    job: Job,
    cancel: CancellationToken,
    /// Completion at the last published progress event.
    published_ratio: Option<f64>,
    /// Monotonic counter for eviction order.
    sequence: u64,
}

#[derive(Debug, Default)]
struct Inner {
    entries: HashMap<JobId, Entry>,
    next_sequence: u64,
}

/// The job registry. Cloning shares the same jobs.
#[derive(Debug, Clone)]
pub struct Jobs {
    inner: Arc<RwLock<Inner>>,
    events: EventBus,
    clock: Arc<dyn Clock>,
}

/// A job's own return value, obtained from [`Jobs::spawn_awaitable`] or
/// [`Jobs::spawn_exclusive_awaitable`].
///
/// Separate from [`Job`], the registry's shared status record: this is process-local
/// and typed per call, for a caller that needs what the job actually produced rather
/// than just whether it finished.
#[derive(Debug)]
pub struct JobResult<T> {
    id: JobId,
    rx: tokio::sync::oneshot::Receiver<Result<T>>,
}

impl<T> JobResult<T> {
    pub fn id(&self) -> &JobId {
        &self.id
    }

    /// Waits for the job to finish and returns what its body returned.
    ///
    /// A job that observed cancellation and returned `Err` surfaces that error here
    /// exactly as the body produced it — cancellation reaches a waiter the same way any
    /// other failure would, there is no separate "was it cancelled" case to handle.
    pub async fn wait(self) -> Result<T> {
        self.rx
            .await
            .unwrap_or_else(|_| Err(AppError::internal("job ended without producing a result")))
    }
}

impl Jobs {
    pub fn new(events: EventBus, clock: Arc<dyn Clock>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner::default())),
            events,
            clock,
        }
    }

    /// Start a job and return immediately.
    ///
    /// A job that panics is recorded as failed rather than taking down the runtime or
    /// silently disappearing — a job the UI shows as running forever is worse than one
    /// that reports an error.
    pub fn spawn<F, Fut>(&self, kind: impl Into<String>, body: F) -> JobId
    where
        F: FnOnce(JobContext) -> Fut + Send + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let (id, _result) = self
            .spawn_core(kind.into(), false, body)
            .expect("spawn without exclusivity never fails");
        id
    }

    /// Like [`Jobs::spawn`], but refuses to start if a job of the same `kind` is
    /// already running.
    ///
    /// `Jobs` has no other notion of "only one at a time": exclusivity is scoped to
    /// `kind`, not the whole registry, and a caller submitting a duplicate request
    /// (the user double-clicking "run", a retry racing the original) gets a clear
    /// error back instead of two jobs quietly running at once.
    pub fn spawn_exclusive<F, Fut>(&self, kind: impl Into<String>, body: F) -> Result<JobId>
    where
        F: FnOnce(JobContext) -> Fut + Send + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.spawn_core(kind.into(), true, body)
            .map(|(id, _result)| id)
    }

    /// Like [`Jobs::spawn`], but the body returns a value the caller can wait for.
    ///
    /// `Job` (what [`Jobs::get`]/[`Jobs::list`] return) is deliberately kind-agnostic
    /// and IPC-safe — a status enum, progress, an error string — with no slot for a
    /// typed result. A caller that needs the job's own output (an audit report, an
    /// export path) rather than just knowing it finished awaits the returned
    /// [`JobResult`] instead of polling or subscribing to events.
    pub fn spawn_awaitable<F, Fut, T>(
        &self,
        kind: impl Into<String>,
        body: F,
    ) -> (JobId, JobResult<T>)
    where
        F: FnOnce(JobContext) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        let (id, result) = self
            .spawn_core(kind.into(), false, body)
            .expect("spawn_awaitable without exclusivity never fails");
        (id, result)
    }

    /// [`Jobs::spawn_exclusive`] and [`Jobs::spawn_awaitable`] combined: only one job
    /// of this `kind` at a time, and the caller can wait for its return value.
    pub fn spawn_exclusive_awaitable<F, Fut, T>(
        &self,
        kind: impl Into<String>,
        body: F,
    ) -> Result<(JobId, JobResult<T>)>
    where
        F: FnOnce(JobContext) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        self.spawn_core(kind.into(), true, body)
    }

    /// Shared machinery behind all four `spawn*` methods.
    ///
    /// One critical section does the exclusivity check and the registry insert
    /// together — checking and inserting under separate locks would let two
    /// `spawn_exclusive` calls race each other between the check and the insert.
    fn spawn_core<F, Fut, T>(
        &self,
        kind: String,
        exclusive: bool,
        body: F,
    ) -> Result<(JobId, JobResult<T>)>
    where
        F: FnOnce(JobContext) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        let job = Job::queued(kind.clone(), self.clock.now());
        let id = job.id.clone();
        let cancel = CancellationToken::new();

        {
            let mut inner = self
                .inner
                .write()
                .unwrap_or_else(|error| error.into_inner());

            if exclusive
                && inner
                    .entries
                    .values()
                    .any(|entry| entry.job.kind == kind && !entry.job.status.is_terminal())
            {
                return Err(AppError::validation(format!(
                    "a '{kind}' job is already running"
                )));
            }

            let sequence = inner.next_sequence;
            inner.next_sequence += 1;
            inner.entries.insert(
                id.clone(),
                Entry {
                    job,
                    cancel: cancel.clone(),
                    published_ratio: None,
                    sequence,
                },
            );
        }

        let (tx, rx) = tokio::sync::oneshot::channel();
        let registry = self.clone();
        let context = JobContext::new(id.clone(), registry.clone(), cancel.clone());
        let started_id = id.clone();

        tokio::spawn(async move {
            registry.mark_started(&started_id, kind.clone());

            // The inner task isolates a panic: awaiting its handle turns the panic into
            // a JoinError we can record.
            let outcome = tokio::spawn(async move { body(context).await }).await;

            let (status, error_message, result): (JobStatus, Option<String>, Result<T>) =
                match outcome {
                    Ok(Ok(value)) if cancel.is_cancelled() => {
                        (JobStatus::Cancelled, None, Ok(value))
                    }
                    Ok(Ok(value)) => (JobStatus::Succeeded, None, Ok(value)),
                    Ok(Err(error)) => {
                        let message = error.to_string();
                        (JobStatus::Failed, Some(message), Err(error))
                    }
                    Err(join_error) if join_error.is_cancelled() => (
                        JobStatus::Cancelled,
                        None,
                        Err(AppError::internal("job cancelled")),
                    ),
                    Err(_) => (
                        JobStatus::Failed,
                        Some("the job panicked".to_owned()),
                        Err(AppError::internal("the job panicked")),
                    ),
                };

            registry.mark_finished(&started_id, kind, status, error_message);
            // No one has to await a JobResult — dropping the receiver is fine.
            let _ = tx.send(result);
        });

        Ok((id.clone(), JobResult { id, rx }))
    }

    pub async fn get(&self, id: &JobId) -> Option<Job> {
        self.inner
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .entries
            .get(id)
            .map(|e| e.job.clone())
    }

    /// Every known job, newest first.
    pub async fn list(&self) -> Vec<Job> {
        let inner = self.inner.read().unwrap_or_else(|error| error.into_inner());
        let mut entries: Vec<&Entry> = inner.entries.values().collect();
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.sequence));
        entries.into_iter().map(|entry| entry.job.clone()).collect()
    }

    pub async fn running(&self) -> Vec<Job> {
        self.list()
            .await
            .into_iter()
            .filter(|job| !job.status.is_terminal())
            .collect()
    }

    /// Request cancellation.
    ///
    /// Returns once the request is recorded, not once the job stopped — a job decides
    /// itself when it can stop safely.
    pub async fn cancel(&self, id: &JobId) -> Result<()> {
        let inner = self.inner.read().unwrap_or_else(|error| error.into_inner());
        let entry = inner
            .entries
            .get(id)
            .ok_or_else(|| AppError::validation(format!("unknown job {id}")))?;

        if !entry.job.cancelable {
            return Err(AppError::validation(format!(
                "job {id} cannot be cancelled"
            )));
        }

        entry.cancel.cancel();
        tracing::debug!(job_id = %id, "cancellation requested");
        Ok(())
    }

    fn mark_started(&self, id: &JobId, kind: String) {
        {
            let mut inner = self
                .inner
                .write()
                .unwrap_or_else(|error| error.into_inner());
            let Some(entry) = inner.entries.get_mut(id) else {
                return;
            };
            entry.job.status = JobStatus::Running;
            entry.job.started_at = self.clock.now();
        }

        let _ = self.events.publish(PlatformEvent::JobStarted(JobStarted {
            job: id.clone(),
            kind,
        }));
    }

    pub(crate) async fn report_progress(&self, id: &JobId, current: u64, total: Option<u64>) {
        let should_publish = {
            let mut inner = self
                .inner
                .write()
                .unwrap_or_else(|error| error.into_inner());
            let Some(entry) = inner.entries.get_mut(id) else {
                return;
            };

            entry.job.progress = Progress { current, total };
            let ratio = entry.job.progress.ratio();

            // Publish when the total first becomes known, and whenever completion moved
            // far enough to be worth a repaint.
            let publish = match (entry.published_ratio, ratio) {
                (None, _) => true,
                (Some(_), None) => false,
                (Some(previous), Some(now)) => (now - previous).abs() >= PROGRESS_STEP,
            };

            if publish {
                entry.published_ratio = ratio.or(entry.published_ratio).or(Some(0.0));
            }
            publish
        };

        if should_publish {
            let _ = self.events.publish(PlatformEvent::JobProgress(JobProgress {
                job: id.clone(),
                current,
                total,
            }));
        }
    }

    fn mark_finished(&self, id: &JobId, kind: String, status: JobStatus, error: Option<String>) {
        {
            let mut inner = self
                .inner
                .write()
                .unwrap_or_else(|error| error.into_inner());
            if let Some(entry) = inner.entries.get_mut(id) {
                entry.job.status = status;
                entry.job.finished_at = Some(self.clock.now());
                entry.job.error = error.clone();
                entry.job.cancelable = false;
            }
            Self::evict_old_finished(&mut inner);
        }

        match status {
            JobStatus::Failed => tracing::warn!(job_id = %id, kind, ?error, "job failed"),
            _ => tracing::debug!(job_id = %id, kind, ?status, "job finished"),
        }

        let _ = self.events.publish(PlatformEvent::JobFinished(JobFinished {
            job: id.clone(),
            kind,
            status,
            error,
        }));
    }

    /// Drop the oldest finished jobs beyond the cap. Running jobs are never evicted.
    fn evict_old_finished(inner: &mut Inner) {
        let mut finished: Vec<(JobId, u64)> = inner
            .entries
            .iter()
            .filter(|(_, entry)| entry.job.status.is_terminal())
            .map(|(id, entry)| (id.clone(), entry.sequence))
            .collect();

        if finished.len() <= MAX_FINISHED {
            return;
        }

        finished.sort_by_key(|(_, sequence)| *sequence);
        for (id, _) in finished.iter().take(finished.len() - MAX_FINISHED) {
            inner.entries.remove(id);
        }
    }
}
