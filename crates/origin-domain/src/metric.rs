//! Metrics as a neutral cross-cutting concept: a number, a unit, a point in time.

use serde::{Deserialize, Serialize};
use std::fmt;
use time::OffsetDateTime;

/// Namespaced metric key, e.g. `github.open_pull_requests`.
///
/// No `#[serde(transparent)]`: for a single-field tuple struct, serde_json already
/// serialises as the bare inner value, and ts-rs cannot parse the attribute.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MetricKey(String);

impl MetricKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MetricKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Count,
    Percent,
    Bytes,
    Milliseconds,
    PerMinute,
    /// Anything the platform does not need to understand.
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Metric {
    pub key: MetricKey,
    pub value: f64,
    pub unit: Unit,
    #[serde(with = "time::serde::rfc3339")]
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub at: OffsetDateTime,
}

impl Metric {
    pub fn new(key: MetricKey, value: f64, unit: Unit, at: OffsetDateTime) -> Self {
        Self {
            key,
            value,
            unit,
            at,
        }
    }
}

/// Comparison of a metric against an earlier period.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Trend {
    pub current: f64,
    pub previous: f64,
}

impl Trend {
    pub fn new(current: f64, previous: f64) -> Self {
        Self { current, previous }
    }

    /// Relative change, e.g. `-0.4` for a 40 % drop.
    ///
    /// Returns `None` when the previous value is zero — there is no meaningful
    /// percentage change from nothing, and reporting `+∞ %` in the UI is worse than
    /// reporting nothing.
    pub fn change_ratio(&self) -> Option<f64> {
        if self.previous == 0.0 {
            None
        } else {
            Some((self.current - self.previous) / self.previous)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_ratio_reports_a_drop() {
        let trend = Trend::new(60.0, 100.0);
        assert_eq!(trend.change_ratio(), Some(-0.4));
    }

    #[test]
    fn change_ratio_from_zero_is_undefined() {
        assert_eq!(Trend::new(10.0, 0.0).change_ratio(), None);
    }
}
