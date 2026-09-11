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

/// Why a sync target is being held back beyond its policy cadence.
///
/// A service-imposed throttle has different origins but is always handled the same
/// way by the engine: the next run may not start before a server-chosen instant.
/// Distinguishing the reason is for logs and the status view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ThrottleReason {
    /// Remaining quota or cost the service reported in the *body*, not a header
    /// (GA4 property quotas, GitHub/Cloudflare GraphQL cost).
    Quota,
    /// A minimum poll interval the service named (e.g. GitHub's `X-Poll-Interval`).
    ServerInterval,
    /// The service rejected a request as rate-limited and named a retry delay.
    RateLimited,
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
    /// A server-imposed floor on the next run: the next run may not start before
    /// this instant, whatever the policy cadence says. Surfaces a quota reset or a
    /// minimum poll interval and survives restart.
    #[serde(with = "time::serde::rfc3339::option")]
    #[cfg_attr(feature = "ts", ts(type = "string | null"))]
    pub not_before: Option<OffsetDateTime>,
    /// Why the target is throttled, if it is. For logs and the status view.
    pub throttle_reason: Option<ThrottleReason>,
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

    /// Set a server-imposed floor on the next run.
    pub fn throttle_until(&mut self, until: OffsetDateTime, reason: ThrottleReason) {
        self.not_before = Some(until);
        self.throttle_reason = Some(reason);
    }

    /// Clear any server-imposed floor.
    pub fn clear_throttle(&mut self) {
        self.not_before = None;
        self.throttle_reason = None;
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

    #[test]
    fn a_throttle_round_trips_and_can_be_cleared() {
        let mut state = SyncState::default();
        let until = datetime!(2026-08-23 10:30 UTC);

        state.throttle_until(until, ThrottleReason::Quota);
        assert_eq!(state.not_before, Some(until));
        assert_eq!(state.throttle_reason, Some(ThrottleReason::Quota));
        assert!(state.not_before.is_some());

        state.clear_throttle();
        assert_eq!(state.not_before, None);
        assert_eq!(state.throttle_reason, None);
    }
}
