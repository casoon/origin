//! Test doubles for the ports defined in this crate.
//!
//! Enabled with the `testing` feature so that production builds do not ship them.

use crate::clock::Clock;
use std::sync::Mutex;
use time::{Duration, OffsetDateTime};

/// A clock that only moves when a test tells it to.
#[derive(Debug)]
pub struct FakeClock {
    now: Mutex<OffsetDateTime>,
}

impl FakeClock {
    pub fn new(now: OffsetDateTime) -> Self {
        Self {
            now: Mutex::new(now),
        }
    }

    /// Move time forward.
    pub fn advance(&self, by: Duration) {
        let mut now = self.now.lock().expect("fake clock poisoned");
        *now += by;
    }

    pub fn set(&self, to: OffsetDateTime) {
        *self.now.lock().expect("fake clock poisoned") = to;
    }
}

impl Clock for FakeClock {
    fn now(&self) -> OffsetDateTime {
        *self.now.lock().expect("fake clock poisoned")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn advancing_moves_the_clock() {
        let clock = FakeClock::new(datetime!(2026-08-23 10:00 UTC));
        clock.advance(Duration::minutes(30));
        assert_eq!(clock.now(), datetime!(2026-08-23 10:30 UTC));
    }
}
