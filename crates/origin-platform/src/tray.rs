//! System-tray contract (G10).
//!
//! Products fill the tray at runtime; the host owns the native widget. Like
//! `NotificationService`, every update is a fire-and-forget call — the
//! implementation decides how a title or badge is rendered.

use async_trait::async_trait;
use origin_domain::Result;
use std::fmt::Debug;

/// A single tray menu entry, product-shaped so the host can build a native menu
/// from it without knowing the product's ids in advance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayMenuItem {
    /// Stable identifier, e.g. `"show"`, `"sync_now"`, `"open_settings"`.
    pub id: String,
    pub label: String,
    /// Whether the item is selectable right now.
    pub enabled: bool,
}

impl TrayMenuItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            enabled: true,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// What the tray icon signals without the user clicking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayBadge {
    None,
    /// A simple red dot — "something needs your attention".
    Attention,
    /// A count, e.g. unread notifications or active alerts.
    Count(u32),
}

/// A product-facing handle to the system tray.
///
/// Present only when the product declared `tray = true`; absent otherwise. A
/// headless or CLI build gets a no-op, so calling `set_title` never panics.
#[async_trait]
pub trait TrayService: Debug + Send + Sync + 'static {
    /// Change the tray tooltip — the text shown on hover.
    async fn set_title(&self, title: &str) -> Result<()>;

    /// Replace the badge on the tray icon.
    async fn set_badge(&self, badge: TrayBadge) -> Result<()>;

    /// Replace the menu. Items are shown in the order given.
    ///
    /// The host rebuilds the native menu from these items and maps selection
    /// to a host-specific handler — the product never sees a pointer or a
    /// platform menu object.
    async fn set_menu(&self, items: Vec<TrayMenuItem>) -> Result<()>;
}

/// Does nothing. For headless runs, CLI builds, and tests.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopTrayService;

#[async_trait]
impl TrayService for NoopTrayService {
    async fn set_title(&self, title: &str) -> Result<()> {
        tracing::debug!(title, "tray title — dropped (noop service)");
        Ok(())
    }

    async fn set_badge(&self, badge: TrayBadge) -> Result<()> {
        tracing::debug!(?badge, "tray badge — dropped (noop service)");
        Ok(())
    }

    async fn set_menu(&self, items: Vec<TrayMenuItem>) -> Result<()> {
        tracing::debug!(count = items.len(), "tray menu — dropped (noop service)");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_noop_service_accepts_everything_without_error() {
        let tray: &dyn TrayService = &NoopTrayService;

        tray.set_title("demo").await.unwrap();
        tray.set_badge(TrayBadge::Count(3)).await.unwrap();
        tray.set_menu(vec![
            TrayMenuItem::new("show", "Show window"),
            TrayMenuItem::new("quit", "Quit"),
        ])
        .await
        .unwrap();
    }
}
