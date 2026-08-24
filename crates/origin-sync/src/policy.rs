use crate::Backoff;
use time::Duration;

/// When a target should be synchronised.
///
/// The same engine serves a one-minute notification poll and a six-hour analytics
/// refresh — only these numbers differ.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyncPolicy {
    /// Cadence while everything is healthy.
    pub interval: Duration,

    /// Floor between two runs, however often a refresh is requested.
    ///
    /// Protects against a user holding the refresh button and against a UI that
    /// re-syncs on every window focus.
    pub min_interval: Duration,

    /// How to back off after failures.
    pub backoff: Backoff,

    /// Retry delay while the machine appears to be offline.
    ///
    /// Short and flat rather than exponential: connectivity usually returns in one
    /// step, and backing off for half an hour would leave the app stale long after
    /// the network came back.
    pub offline_retry: Duration,
}

impl Default for SyncPolicy {
    fn default() -> Self {
        Self {
            interval: Duration::minutes(5),
            min_interval: Duration::seconds(30),
            backoff: Backoff::default(),
            offline_retry: Duration::seconds(20),
        }
    }
}

impl SyncPolicy {
    /// A policy that runs every `interval`.
    pub fn every(interval: Duration) -> Self {
        Self {
            interval,
            // A floor of a tenth of the cadence, but never below five seconds.
            min_interval: (interval / 10_i32).max(Duration::seconds(5)),
            ..Self::default()
        }
    }

    pub fn with_min_interval(mut self, min_interval: Duration) -> Self {
        self.min_interval = min_interval;
        self
    }

    pub fn with_backoff(mut self, backoff: Backoff) -> Self {
        self.backoff = backoff;
        self
    }

    pub fn with_offline_retry(mut self, offline_retry: Duration) -> Self {
        self.offline_retry = offline_retry;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fast_cadence_still_keeps_a_sane_floor() {
        assert_eq!(
            SyncPolicy::every(Duration::seconds(10)).min_interval,
            Duration::seconds(5)
        );
    }

    #[test]
    fn a_slow_cadence_scales_its_floor() {
        assert_eq!(
            SyncPolicy::every(Duration::hours(1)).min_interval,
            Duration::minutes(6)
        );
    }
}
