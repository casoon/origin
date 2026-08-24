use async_trait::async_trait;
use origin_core::Result;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    pub title: String,
    pub body: Option<String>,
    pub urgency: Urgency,
    /// Groups related notifications so a repeat replaces the previous one instead of
    /// stacking. Usually the alert fingerprint.
    pub tag: Option<String>,
}

impl Notification {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: None,
            urgency: Urgency::Normal,
            tag: None,
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
