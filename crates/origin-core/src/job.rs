//! Background jobs. Everything long-running reports progress the same way, so every
//! product can reuse one progress UI.

use crate::ids::JobId;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Progress {
    pub current: u64,
    /// `None` while the total is not yet known — the UI shows an indeterminate bar.
    pub total: Option<u64>,
}

impl Progress {
    pub fn indeterminate() -> Self {
        Self {
            current: 0,
            total: None,
        }
    }

    pub fn of(current: u64, total: u64) -> Self {
        Self {
            current,
            total: Some(total),
        }
    }

    /// Completion between `0.0` and `1.0`, if the total is known and non-zero.
    pub fn ratio(&self) -> Option<f64> {
        match self.total {
            Some(total) if total > 0 => Some((self.current as f64 / total as f64).clamp(0.0, 1.0)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Job {
    pub id: JobId,
    /// Product-defined job kind, e.g. `scan-repository`.
    pub kind: String,
    pub status: JobStatus,
    pub progress: Progress,
    pub cancelable: bool,
    #[serde(with = "time::serde::rfc3339")]
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    #[cfg_attr(feature = "ts", ts(type = "string | null"))]
    pub finished_at: Option<OffsetDateTime>,
    pub error: Option<String>,
}

impl Job {
    pub fn queued(kind: impl Into<String>, started_at: OffsetDateTime) -> Self {
        Self {
            id: JobId::generate(),
            kind: kind.into(),
            status: JobStatus::Queued,
            progress: Progress::indeterminate(),
            cancelable: true,
            started_at,
            finished_at: None,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_is_none_while_the_total_is_unknown() {
        assert_eq!(Progress::indeterminate().ratio(), None);
    }

    #[test]
    fn ratio_never_exceeds_one() {
        assert_eq!(Progress::of(12, 10).ratio(), Some(1.0));
    }
}
