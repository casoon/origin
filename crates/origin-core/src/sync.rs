//! Synchronisation bookkeeping.
//!
//! The connector decides *how* to fetch. This type records what happened, so the
//! platform can decide *when* to try again.

use crate::error::ErrorKind;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum SyncOutcome {
    /// New data was fetched and stored.
    Updated,
    /// The service reported no change (ETag / Last-Modified hit).
    NotModified,
    Failed {
        kind: ErrorKind,
        message: String,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SyncState {
    #[serde(with = "time::serde::rfc3339::option")]
    #[cfg_attr(feature = "ts", ts(type = "string | null"))]
    pub last_attempt: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    #[cfg_attr(feature = "ts", ts(type = "string | null"))]
    pub last_success: Option<OffsetDateTime>,
    pub last_outcome: Option<SyncOutcome>,
    /// Validators handed back to the service on the next request.
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    /// Consecutive failures, used for exponential backoff.
    pub failure_streak: u32,
}

impl SyncState {
    pub fn record(&mut self, at: OffsetDateTime, outcome: SyncOutcome) {
        self.last_attempt = Some(at);
        match &outcome {
            SyncOutcome::Updated | SyncOutcome::NotModified => {
                self.last_success = Some(at);
                self.failure_streak = 0;
            }
            SyncOutcome::Failed { .. } => {
                self.failure_streak = self.failure_streak.saturating_add(1);
            }
        }
        self.last_outcome = Some(outcome);
    }

    pub fn is_failing(&self) -> bool {
        self.failure_streak > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn a_not_modified_response_still_counts_as_success() {
        let mut state = SyncState::default();
        state.record(
            datetime!(2026-08-23 10:00 UTC),
            SyncOutcome::Failed {
                kind: ErrorKind::Network,
                message: "timeout".into(),
            },
        );
        assert_eq!(state.failure_streak, 1);

        state.record(datetime!(2026-08-23 10:05 UTC), SyncOutcome::NotModified);

        assert_eq!(state.failure_streak, 0);
        assert_eq!(state.last_success, Some(datetime!(2026-08-23 10:05 UTC)));
    }
}
