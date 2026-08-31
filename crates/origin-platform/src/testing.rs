//! Test doubles for the platform contracts.

use crate::{Notification, NotificationService, Opener};
use async_trait::async_trait;
use origin_domain::Result;
use std::sync::Mutex;

/// Records notifications instead of showing them, so tests can assert on what the
/// user *would* have seen.
#[derive(Debug, Default)]
pub struct RecordingNotificationService {
    sent: Mutex<Vec<Notification>>,
}

impl RecordingNotificationService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sent(&self) -> Vec<Notification> {
        self.sent.lock().expect("recorder poisoned").clone()
    }
}

#[async_trait]
impl NotificationService for RecordingNotificationService {
    async fn notify(&self, notification: Notification) -> Result<()> {
        self.sent
            .lock()
            .expect("recorder poisoned")
            .push(notification);
        Ok(())
    }
}

/// Records opened URLs instead of launching a browser.
#[derive(Debug, Default)]
pub struct RecordingOpener {
    opened: Mutex<Vec<String>>,
}

impl RecordingOpener {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn opened(&self) -> Vec<String> {
        self.opened.lock().expect("recorder poisoned").clone()
    }
}

#[async_trait]
impl Opener for RecordingOpener {
    async fn open_url(&self, url: &str) -> Result<()> {
        self.opened
            .lock()
            .expect("recorder poisoned")
            .push(url.to_owned());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_recorder_captures_what_the_user_would_have_seen() {
        let notifications = RecordingNotificationService::new();
        notifications
            .notify(Notification::new("CI failed").with_tag("ci:main"))
            .await
            .unwrap();

        let sent = notifications.sent();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].title, "CI failed");
        assert_eq!(sent[0].tag.as_deref(), Some("ci:main"));
    }
}
