//! Events every Origin application shares.
//!
//! Products add their own enums (`GitHubEvent`, `AnalyticsEvent`, ...) and publish
//! them on the same bus.

use crate::bus::Event;
use origin_core::{AccountId, Alert, AlertId, ConnectorId, ErrorKind, JobId, JobStatus, SyncId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SyncCompleted {
    pub sync: SyncId,
    pub connector: ConnectorId,
    pub account: AccountId,
    /// How many records changed. `0` means the service reported no change.
    pub changed: u64,
    #[serde(with = "time::serde::rfc3339")]
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SyncFailed {
    pub sync: SyncId,
    pub connector: ConnectorId,
    pub account: AccountId,
    pub kind: ErrorKind,
    pub message: String,
    /// When the platform intends to try again, if it does.
    #[serde(with = "time::serde::rfc3339::option")]
    #[cfg_attr(feature = "ts", ts(type = "string | null"))]
    pub retry_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AlertRaised {
    pub alert: Alert,
    /// `true` when an alert with the same fingerprint was already active, so
    /// notification sinks can stay quiet.
    pub deduplicated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AlertResolved {
    pub alert: AlertId,
    #[serde(with = "time::serde::rfc3339")]
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AccountExpired {
    pub account: AccountId,
    pub connector: ConnectorId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct JobStarted {
    pub job: JobId,
    pub kind: String,
}

/// Progress of a running job.
///
/// Deliberately throttled by the job registry: a job that reports every one of ten
/// thousand steps would flood the bus and make slow subscribers lag, losing the
/// *finished* event they actually care about.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct JobProgress {
    pub job: JobId,
    pub current: u64,
    pub total: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct JobFinished {
    pub job: JobId,
    pub kind: String,
    pub status: JobStatus,
    pub error: Option<String>,
}

/// The platform-level event enum.
///
/// Adding a variant is a breaking change for exhaustive subscribers — deliberately so.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlatformEvent {
    SyncCompleted(SyncCompleted),
    SyncFailed(SyncFailed),
    AlertRaised(AlertRaised),
    AlertResolved(AlertResolved),
    AccountExpired(AccountExpired),
    JobStarted(JobStarted),
    JobProgress(JobProgress),
    JobFinished(JobFinished),
}

impl Event for PlatformEvent {
    fn name(&self) -> &'static str {
        match self {
            Self::SyncCompleted(_) => "platform.sync.completed",
            Self::SyncFailed(_) => "platform.sync.failed",
            Self::AlertRaised(_) => "platform.alert.raised",
            Self::AlertResolved(_) => "platform.alert.resolved",
            Self::AccountExpired(_) => "platform.account.expired",
            Self::JobStarted(_) => "platform.job.started",
            Self::JobProgress(_) => "platform.job.progress",
            Self::JobFinished(_) => "platform.job.finished",
        }
    }
}
