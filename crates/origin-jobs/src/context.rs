use crate::registry::Jobs;
use origin_domain::JobId;
use tokio_util::sync::CancellationToken;

/// What a running job is handed.
///
/// It is the only way a job reports progress or learns that it should stop, which keeps
/// job bodies free of any registry or event-bus knowledge.
#[derive(Debug, Clone)]
pub struct JobContext {
    id: JobId,
    jobs: Jobs,
    cancel: CancellationToken,
}

impl JobContext {
    pub(crate) fn new(id: JobId, jobs: Jobs, cancel: CancellationToken) -> Self {
        Self { id, jobs, cancel }
    }

    pub fn id(&self) -> &JobId {
        &self.id
    }

    /// Report progress. `total` may be `None` while it is still unknown.
    ///
    /// Cheap to call in a tight loop: the registry decides what is worth publishing.
    pub async fn progress(&self, current: u64, total: Option<u64>) {
        self.jobs.report_progress(&self.id, current, total).await;
    }

    /// Whether cancellation was requested. Check this between units of work.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// Resolves when cancellation is requested.
    ///
    /// For jobs that wait on something instead of looping:
    /// `tokio::select! { _ = ctx.cancelled() => …, result = work => … }`.
    pub async fn cancelled(&self) {
        self.cancel.cancelled().await;
    }
}
