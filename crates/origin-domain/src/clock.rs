//! Time as a port.
//!
//! Domain code never calls `OffsetDateTime::now_utc()` directly — otherwise anything
//! involving backoff, TTL or scheduling becomes untestable.

use std::fmt::Debug;
use time::OffsetDateTime;

pub trait Clock: Debug + Send + Sync + 'static {
    fn now(&self) -> OffsetDateTime;
}

/// The real system clock, in UTC.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}
