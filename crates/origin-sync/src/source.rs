use crate::SyncTarget;
use async_trait::async_trait;
use origin_domain::{Result, SyncId, SyncState, ThrottleReason};
use std::fmt::Debug;
use time::Duration;
use tokio_util::sync::CancellationToken;

/// A service-imposed limit on how soon the next run may happen.
///
/// The connector reports it; the engine owns scheduling. It arrives in two shapes —
/// a *quota* the service reported in the body, and a *minimum poll interval* — but is
/// handled identically: the next run may not start before `delay` has elapsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncThrottle {
    pub delay: Duration,
    pub reason: ThrottleReason,
}

impl SyncThrottle {
    /// A quota or cost reported in the body, not a header (G6).
    pub fn quota(delay: Duration) -> Self {
        Self {
            delay,
            reason: ThrottleReason::Quota,
        }
    }

    /// A minimum poll interval the service named (G7).
    pub fn server_interval(delay: Duration) -> Self {
        Self {
            delay,
            reason: ThrottleReason::ServerInterval,
        }
    }
}

/// What one successful sync produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncReport {
    /// How many records changed. Used for the "3 new notifications" kind of message.
    pub changed: u64,
    /// Validator to send on the next request, if the service issued one.
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    /// A throttle the service reported alongside the data. `None` keeps the policy
    /// cadence; `Some` pushes the next run back to at least `delay` from now.
    pub throttle: Option<SyncThrottle>,
}

impl SyncReport {
    pub fn changed(changed: u64) -> Self {
        Self {
            changed,
            ..Self::default()
        }
    }

    pub fn with_etag(mut self, etag: impl Into<String>) -> Self {
        self.etag = Some(etag.into());
        self
    }

    pub fn with_last_modified(mut self, last_modified: impl Into<String>) -> Self {
        self.last_modified = Some(last_modified.into());
        self
    }

    pub fn with_throttle(mut self, throttle: SyncThrottle) -> Self {
        self.throttle = Some(throttle);
        self
    }
}

/// The outcome a source reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncResult {
    Updated(SyncReport),
    /// The service confirmed nothing changed — a validator hit. Counts as success.
    NotModified,
}

/// Everything a source is given for one run.
#[derive(Debug)]
pub struct SyncContext {
    pub sync_id: SyncId,
    pub target: SyncTarget,
    /// State from the previous run: validators, failure streak, timestamps.
    pub state: SyncState,
    cancel: CancellationToken,
}

impl SyncContext {
    pub(crate) fn new(
        sync_id: SyncId,
        target: SyncTarget,
        state: SyncState,
        cancel: CancellationToken,
    ) -> Self {
        Self {
            sync_id,
            target,
            state,
            cancel,
        }
    }

    /// Validator from the last successful run, to be sent as `If-None-Match`.
    pub fn etag(&self) -> Option<&str> {
        self.state.etag.as_deref()
    }

    /// Validator from the last successful run, to be sent as `If-Modified-Since`.
    pub fn last_modified(&self) -> Option<&str> {
        self.state.last_modified.as_deref()
    }

    /// Check between pages of a paginated fetch.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    pub async fn cancelled(&self) {
        self.cancel.cancelled().await;
    }
}

/// How a particular kind of data is fetched.
///
/// Implementations answer *how*, never *when*. They do not sleep, do not retry and do
/// not decide whether the machine is online — the engine owns all of that.
#[async_trait]
pub trait SyncSource: Debug + Send + Sync + 'static {
    async fn sync(&self, context: &SyncContext) -> Result<SyncResult>;
}
