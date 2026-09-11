use async_trait::async_trait;
use origin_domain::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    Low,
    #[default]
    Normal,
    Critical,
}

/// One action offered on a notification (B6).
///
/// The action *is* the label plus a stable id: the host renders a button and reports
/// the id back, so a product reacts to `"mark-read"` rather than to a translated label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationAction {
    pub id: String,
    pub label: String,
}

impl NotificationAction {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    pub title: String,
    pub body: Option<String>,
    pub urgency: Urgency,
    /// Groups related notifications so a repeat replaces the previous one instead of
    /// stacking. Usually the alert fingerprint.
    pub tag: Option<String>,
    /// Buttons on the notification itself (B6).
    ///
    /// Recording them is platform-independent; whether a host can *show* them is not
    /// — macOS, for instance, needs the notification category registered up front. A
    /// host that cannot render actions shows the notification without them rather than
    /// failing.
    #[serde(default)]
    pub actions: Vec<NotificationAction>,
}

impl Notification {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: None,
            urgency: Urgency::Normal,
            tag: None,
            actions: Vec::new(),
        }
    }

    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn with_urgency(mut self, urgency: Urgency) -> Self {
        self.urgency = urgency;
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Attach an action button. The host reports `id` back when the user presses it.
    pub fn with_action(mut self, action: NotificationAction) -> Self {
        self.actions.push(action);
        self
    }
}

/// Native user notifications.
///
/// Implementations must not fail the caller when the user has denied notification
/// permission — a suppressed notification is a normal outcome, not an error.
#[async_trait]
pub trait NotificationService: Debug + Send + Sync + 'static {
    async fn notify(&self, notification: Notification) -> Result<()>;
}

/// Drops every notification. Used for headless runs and CLI builds.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopNotificationService;

#[async_trait]
impl NotificationService for NoopNotificationService {
    async fn notify(&self, notification: Notification) -> Result<()> {
        tracing::debug!(title = %notification.title, "notification dropped (noop service)");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_notification_carries_actions_with_stable_ids() {
        let notification = Notification::new("Pull request ready")
            .with_action(NotificationAction::new("open", "Open"))
            .with_action(NotificationAction::new("mark-read", "Mark as read"));

        assert_eq!(notification.actions.len(), 2);
        assert_eq!(notification.actions[0].id, "open");
        assert_eq!(notification.actions[1].id, "mark-read");
    }

    #[test]
    fn a_plain_notification_has_no_actions() {
        assert!(Notification::new("Ping").actions.is_empty());
    }
}
