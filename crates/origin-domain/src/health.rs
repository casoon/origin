//! A neutral health model shared by all products.
//!
//! It deliberately does not try to unify the underlying data — only to make different
//! systems comparable in the UI.

use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Health {
    Healthy,
    Warning,
    Critical,
    /// Nothing has been observed yet, or the last observation is too old to trust.
    #[default]
    Unknown,
}

impl Health {
    /// Severity ranking for aggregation. Higher wins.
    fn rank(self) -> u8 {
        match self {
            Self::Healthy => 0,
            Self::Unknown => 1,
            Self::Warning => 2,
            Self::Critical => 3,
        }
    }

    /// The more alarming of two states.
    pub fn worse(self, other: Self) -> Self {
        if other.rank() > self.rank() {
            other
        } else {
            self
        }
    }

    /// Aggregate many states into one. An empty iterator is [`Health::Unknown`].
    pub fn aggregate(items: impl IntoIterator<Item = Self>) -> Self {
        items
            .into_iter()
            .fold(None, |acc: Option<Self>, item| {
                Some(acc.map_or(item, |current| current.worse(item)))
            })
            .unwrap_or(Self::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critical_dominates_every_other_state() {
        let aggregate = Health::aggregate([Health::Healthy, Health::Critical, Health::Warning]);
        assert_eq!(aggregate, Health::Critical);
    }

    #[test]
    fn unknown_outranks_healthy_but_not_warning() {
        assert_eq!(
            Health::aggregate([Health::Healthy, Health::Unknown]),
            Health::Unknown
        );
        assert_eq!(
            Health::aggregate([Health::Unknown, Health::Warning]),
            Health::Warning
        );
    }

    #[test]
    fn empty_aggregation_is_unknown() {
        assert_eq!(Health::aggregate([]), Health::Unknown);
    }
}
