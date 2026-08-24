use crate::{SyncPolicy, SyncTarget};
use origin_core::{Health, SyncState};
use serde::Serialize;
use time::OffsetDateTime;

/// One registered target and how it is doing — the shape the frontend renders.
///
/// Defined here rather than in the host layer: it is a contract type, and contract
/// types belong with the domain they describe, not with the transport that carries
/// them.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SyncStatus {
    pub target: SyncTarget,
    pub state: SyncState,
    pub health: Health,
    /// When the engine intends to run it next, RFC 3339.
    pub due_at: Option<String>,
}

/// Failures before a target is considered critical rather than merely unhappy.
const CRITICAL_STREAK: u32 = 3;

/// How many intervals a target may go without a successful run before it counts as
/// stale, even while nothing is visibly failing.
const STALE_INTERVALS: i32 = 3;

/// Translate sync bookkeeping into the shared health model.
///
/// Deliberately not part of `SyncState`: what counts as healthy depends on the
/// cadence, and only the policy knows that.
pub fn health_of(state: &SyncState, policy: &SyncPolicy, now: OffsetDateTime) -> Health {
    if state.failure_streak >= CRITICAL_STREAK {
        return Health::Critical;
    }

    match state.last_success {
        // Never succeeded: unknown while untried, a warning once it has failed.
        None if state.failure_streak == 0 => Health::Unknown,
        None => Health::Warning,

        Some(last_success) => {
            let stale = now - last_success > policy.interval * STALE_INTERVALS;

            if state.failure_streak > 0 || stale {
                Health::Warning
            } else {
                Health::Healthy
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use origin_core::{ErrorKind, SyncOutcome};
    use time::Duration;
    use time::macros::datetime;

    const NOW: OffsetDateTime = datetime!(2026-08-23 10:00 UTC);

    fn policy() -> SyncPolicy {
        SyncPolicy::every(Duration::minutes(5))
    }

    fn failed(state: &mut SyncState, times: u32) {
        for _ in 0..times {
            state.record(
                NOW,
                SyncOutcome::Failed {
                    kind: ErrorKind::Network,
                    message: "timeout".into(),
                },
            );
        }
    }

    #[test]
    fn a_target_that_never_ran_is_unknown_not_broken() {
        assert_eq!(
            health_of(&SyncState::default(), &policy(), NOW),
            Health::Unknown
        );
    }

    #[test]
    fn a_fresh_success_is_healthy() {
        let mut state = SyncState::default();
        state.record(NOW, SyncOutcome::Updated);

        assert_eq!(health_of(&state, &policy(), NOW), Health::Healthy);
    }

    #[test]
    fn a_single_failure_is_a_warning_not_a_crisis() {
        let mut state = SyncState::default();
        state.record(NOW, SyncOutcome::Updated);
        failed(&mut state, 1);

        assert_eq!(health_of(&state, &policy(), NOW), Health::Warning);
    }

    #[test]
    fn repeated_failures_become_critical() {
        let mut state = SyncState::default();
        failed(&mut state, CRITICAL_STREAK);

        assert_eq!(health_of(&state, &policy(), NOW), Health::Critical);
    }

    #[test]
    fn silence_is_reported_even_when_nothing_visibly_failed() {
        let mut state = SyncState::default();
        state.record(NOW, SyncOutcome::Updated);

        let much_later = NOW + Duration::minutes(5) * STALE_INTERVALS + Duration::seconds(1);

        assert_eq!(
            health_of(&state, &policy(), much_later),
            Health::Warning,
            "a target that quietly stopped running is not healthy"
        );
    }

    #[test]
    fn a_not_modified_response_keeps_a_target_healthy() {
        let mut state = SyncState::default();
        state.record(NOW, SyncOutcome::NotModified);

        assert_eq!(health_of(&state, &policy(), NOW), Health::Healthy);
    }
}
