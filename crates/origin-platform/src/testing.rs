//! Recording test doubles for the platform contracts.

use crate::confirmation::{ConfirmationDecision, ConfirmationRequest, ConfirmationService};
use crate::notifications::{Notification, NotificationService};
use crate::opener::Opener;
use crate::tray::{TrayBadge, TrayMenuItem, TrayService};
use async_trait::async_trait;
use origin_domain::Result;
use std::collections::VecDeque;
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

/// Records confirmation requests and answers from a scripted sequence.
///
/// Useful both for asserting *what* was asked and for driving a specific decision.
#[derive(Debug)]
pub struct RecordingConfirmationService {
    decisions: Mutex<VecDeque<ConfirmationDecision>>,
    fallback: ConfirmationDecision,
    requests: Mutex<Vec<ConfirmationRequest>>,
}

impl RecordingConfirmationService {
    /// Approves everything, and records every request.
    pub fn approving() -> Self {
        Self {
            decisions: Mutex::new(VecDeque::new()),
            fallback: ConfirmationDecision::Approved,
            requests: Mutex::new(Vec::new()),
        }
    }

    /// Denies everything, and records every request.
    pub fn denying() -> Self {
        Self {
            decisions: Mutex::new(VecDeque::new()),
            fallback: ConfirmationDecision::Denied,
            requests: Mutex::new(Vec::new()),
        }
    }

    /// Answer the queued decisions front-to-back, then `fallback`.
    pub fn scripted(
        decisions: impl IntoIterator<Item = ConfirmationDecision>,
        fallback: ConfirmationDecision,
    ) -> Self {
        Self {
            decisions: Mutex::new(decisions.into_iter().collect()),
            fallback,
            requests: Mutex::new(Vec::new()),
        }
    }

    /// Requests seen so far, in order.
    pub fn requests(&self) -> Vec<ConfirmationRequest> {
        self.requests.lock().expect("recorder poisoned").clone()
    }
}

#[async_trait]
impl ConfirmationService for RecordingConfirmationService {
    async fn confirm(&self, request: ConfirmationRequest) -> Result<ConfirmationDecision> {
        self.requests
            .lock()
            .expect("recorder poisoned")
            .push(request.clone());

        let mut decisions = self.decisions.lock().expect("recorder poisoned");
        Ok(decisions.pop_front().unwrap_or(self.fallback))
    }
}

/// Records tray updates, so tests can assert what was sent.
#[derive(Debug, Default)]
pub struct RecordingTrayService {
    titles: Mutex<Vec<String>>,
    badges: Mutex<Vec<TrayBadge>>,
    menus: Mutex<Vec<Vec<TrayMenuItem>>>,
}

impl RecordingTrayService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn titles(&self) -> Vec<String> {
        self.titles.lock().expect("recorder poisoned").clone()
    }

    pub fn badges(&self) -> Vec<TrayBadge> {
        self.badges.lock().expect("recorder poisoned").clone()
    }

    pub fn menus(&self) -> Vec<Vec<TrayMenuItem>> {
        self.menus.lock().expect("recorder poisoned").clone()
    }
}

#[async_trait]
impl TrayService for RecordingTrayService {
    async fn set_title(&self, title: &str) -> Result<()> {
        self.titles
            .lock()
            .expect("recorder poisoned")
            .push(title.to_owned());
        Ok(())
    }

    async fn set_badge(&self, badge: TrayBadge) -> Result<()> {
        self.badges.lock().expect("recorder poisoned").push(badge);
        Ok(())
    }

    async fn set_menu(&self, items: Vec<TrayMenuItem>) -> Result<()> {
        self.menus.lock().expect("recorder poisoned").push(items);
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

    #[tokio::test]
    async fn the_confirmation_recorder_returns_scripted_decisions_and_records_requests() {
        let confirmations = RecordingConfirmationService::scripted(
            [ConfirmationDecision::Approved, ConfirmationDecision::Denied],
            ConfirmationDecision::Approved,
        );

        let first = confirmations
            .confirm(ConfirmationRequest::new("one", "first ask"))
            .await
            .unwrap();
        let second = confirmations
            .confirm(ConfirmationRequest::new("two", "second ask"))
            .await
            .unwrap();
        let third = confirmations
            .confirm(ConfirmationRequest::new("three", "fallback ask"))
            .await
            .unwrap();

        assert_eq!(first, ConfirmationDecision::Approved);
        assert_eq!(second, ConfirmationDecision::Denied);
        assert_eq!(
            third,
            ConfirmationDecision::Approved,
            "sequence exhausted → fallback"
        );

        let requests = confirmations.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].title, "one");
        assert_eq!(requests[1].title, "two");
    }
}
