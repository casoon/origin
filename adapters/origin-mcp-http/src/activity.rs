//! Endpoint activity tracking (G18).
//!
//! "An external AI is driving this application" has to be *visible*, not just logged.
//! The endpoint records when it last served a request; a host reads that to raise a
//! tray indicator while the AI is connected and clear it once the endpoint has been
//! quiet for a while.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A cloneable view of when the endpoint last served a request.
#[derive(Debug, Clone, Default)]
pub struct Activity {
    last: Arc<Mutex<Option<Instant>>>,
}

impl Activity {
    /// Record that a request just arrived.
    pub fn touch(&self) {
        *self.last.lock().expect("activity poisoned") = Some(Instant::now());
    }

    /// Whether any request has ever arrived.
    pub fn has_seen_activity(&self) -> bool {
        self.last.lock().expect("activity poisoned").is_some()
    }

    /// Whether the endpoint has been quiet for at least `idle`.
    ///
    /// An endpoint that has never served a request counts as idle.
    pub fn is_idle_for(&self, idle: Duration) -> bool {
        match *self.last.lock().expect("activity poisoned") {
            None => true,
            Some(at) => at.elapsed() >= idle,
        }
    }

    /// Whether a session looks active: activity has been seen and it is recent.
    pub fn is_active(&self, idle: Duration) -> bool {
        self.has_seen_activity() && !self.is_idle_for(idle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_activity_has_seen_nothing_and_is_idle() {
        let activity = Activity::default();

        assert!(!activity.has_seen_activity());
        assert!(activity.is_idle_for(Duration::from_secs(1)));
        assert!(!activity.is_active(Duration::from_secs(60)));
    }

    #[test]
    fn touching_makes_it_active_then_idle_again() {
        let activity = Activity::default();
        activity.touch();

        assert!(activity.has_seen_activity());
        assert!(activity.is_active(Duration::from_secs(60)));
        assert!(!activity.is_idle_for(Duration::from_secs(60)));

        // Any non-zero idle window has elapsed after a sleep.
        std::thread::sleep(Duration::from_millis(20));
        assert!(activity.is_idle_for(Duration::from_millis(10)));
        assert!(!activity.is_active(Duration::from_millis(10)));
    }
}
