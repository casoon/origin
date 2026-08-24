//! Native notifications via `tauri-plugin-notification`.
//!
//! This is the concrete side of [`origin_platform::NotificationService`] — the only
//! place in the notification path that knows Tauri exists (ADR-0001).

use async_trait::async_trait;
use origin_core::{AppError, Result};
use origin_platform::{Notification, NotificationService, Urgency};
use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::{NotificationExt, PermissionState};

#[derive(Debug, Clone)]
pub struct TauriNotificationService<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriNotificationService<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }

    /// Ask the OS for permission if it has not been decided yet.
    ///
    /// Returns whether notifications may be shown.
    fn ensure_permission(&self) -> Result<bool> {
        let state = self
            .app
            .notification()
            .permission_state()
            .map_err(|error| AppError::internal(format!("notification permission: {error}")))?;

        let state = match state {
            PermissionState::Prompt | PermissionState::PromptWithRationale => self
                .app
                .notification()
                .request_permission()
                .map_err(|error| {
                    AppError::internal(format!("notification permission request: {error}"))
                })?,
            decided => decided,
        };

        Ok(matches!(state, PermissionState::Granted))
    }
}

#[async_trait]
impl<R: Runtime> NotificationService for TauriNotificationService<R> {
    async fn notify(&self, notification: Notification) -> Result<()> {
        // A user who declined notifications is not an error condition — the caller
        // (a sync run, an alert) must carry on regardless.
        if !self.ensure_permission()? {
            tracing::debug!(
                title = %notification.title,
                "notification suppressed: permission not granted"
            );
            return Ok(());
        }

        let mut builder = self.app.notification().builder().title(&notification.title);

        if let Some(body) = &notification.body {
            builder = builder.body(body);
        }
        if let Some(tag) = &notification.tag {
            // Replaces an earlier notification with the same tag instead of stacking.
            builder = builder.group(tag);
        }
        if notification.urgency == Urgency::Critical {
            builder = builder.sound("default");
        }

        builder
            .show()
            .map_err(|error| AppError::internal(format!("cannot show notification: {error}")))
    }
}
