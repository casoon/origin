use crate::context::JobContext;
use origin_core::{AppError, Clock, Job, JobId, JobStatus, Progress, Result};
use origin_events::{EventBus, JobFinished, JobProgress, JobStarted, PlatformEvent};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::RwLock;
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
        let kind = kind.into();
        let job = Job::queued(kind.clone(), self.clock.now());
        let id = job.id.clone();
        let cancel = CancellationToken::new();

        let registry = self.clone();
        let context = JobContext::new(id.clone(), registry.clone(), cancel.clone());
        let started_id = id.clone();

        tokio::spawn(async move {
            registry
                .mark_started(&started_id, job, cancel.clone(), kind.clone())
                .await;

            // The inner task isolates a panic: awaiting its handle turns the panic into
            // a JoinError we can record.
            let outcome = tokio::spawn(async move { body(context).await }).await;

            let status_and_error = match outcome {
                Ok(Ok(())) if cancel.is_cancelled() => (JobStatus::Cancelled, None),
                Ok(Ok(())) => (JobStatus::Succeeded, None),
                Ok(Err(error)) => (JobStatus::Failed, Some(error.to_string())),
                Err(join_error) if join_error.is_cancelled() => (JobStatus::Cancelled, None),
                Err(_) => (JobStatus::Failed, Some("the job panicked".to_owned())),
            };

            registry
                .mark_finished(&started_id, kind, status_and_error.0, status_and_error.1)
                .await;
        });

        id
    }

    pub async fn get(&self, id: &JobId) -> Option<Job> {
        self.inner
            .read()
            .await
            .entries
            .get(id)
            .map(|e| e.job.clone())
    }

    /// Every known job, newest first.
    pub async fn list(&self) -> Vec<Job> {
        let inner = self.inner.read().await;
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
        let inner = self.inner.read().await;
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

    async fn mark_started(
        &self,
        id: &JobId,
        mut job: Job,
        cancel: CancellationToken,
        kind: String,
    ) {
        job.status = JobStatus::Running;
        job.started_at = self.clock.now();

        {
            let mut inner = self.inner.write().await;
            let sequence = inner.next_sequence;
            inner.next_sequence += 1;
            inner.entries.insert(
                id.clone(),
                Entry {
                    job,
                    cancel,
                    published_ratio: None,
                    sequence,
                },
            );
        }

        let _ = self.events.publish(PlatformEvent::JobStarted(JobStarted {
            job: id.clone(),
            kind,
        }));
    }

    pub(crate) async fn report_progress(&self, id: &JobId, current: u64, total: Option<u64>) {
        let should_publish = {
            let mut inner = self.inner.write().await;
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

    async fn mark_finished(
        &self,
        id: &JobId,
        kind: String,
        status: JobStatus,
        error: Option<String>,
    ) {
        {
            let mut inner = self.inner.write().await;
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
