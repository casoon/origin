//! Correlation spans.
//!
//! Every long-running platform operation opens one of these, so a log line can be
//! traced back to the sync run, job or account that produced it.

use origin_domain::{AccountId, ConnectorId, JobId, SyncId};
use tracing::Span;

/// Span covering one synchronisation run.
pub fn sync(sync: &SyncId, connector: &ConnectorId, account: &AccountId) -> Span {
    tracing::info_span!(
        "sync",
        sync_id = sync.as_str(),
        connector = connector.as_str(),
        account_id = account.as_str(),
    )
}

/// Span covering one background job.
pub fn job(job: &JobId, kind: &str) -> Span {
    tracing::info_span!("job", job_id = job.as_str(), kind = kind)
}

/// Span covering one outbound request, for rate-limit and latency analysis.
pub fn request(connector: &ConnectorId, operation: &str) -> Span {
    tracing::debug_span!(
        "request",
        connector = connector.as_str(),
        operation = operation
    )
}
