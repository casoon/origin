//! Global shortcut contract (B5).
//!
//! A global shortcut fires while the application is in the background — the "quick
//! capture" pattern. Registration is the product's choice; the host owns the native
//! binding. Like the tray, this is present only when the product declares it.

use async_trait::async_trait;
use origin_domain::Result;
use std::fmt::Debug;

/// One accelerator the product wants to own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    /// Stable id so the product can tell which shortcut fired.
    pub id: String,
    /// Platform-neutral accelerator, e.g. `"CmdOrCtrl+Shift+Space"`.
    pub accelerator: String,
}

impl Shortcut {
    pub fn new(id: impl Into<String>, accelerator: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            accelerator: accelerator.into(),
        }
    }
}

/// Registers global shortcuts.
///
/// The host reports a press as a typed event (ARCHITECTURE.md rule 9), never as a
/// callback the product registers by string. A headless or CLI build gets a no-op.
#[async_trait]
pub trait GlobalShortcutService: Debug + Send + Sync + 'static {
    /// Register `shortcut`, or replace an existing one with the same id.
    async fn register(&self, shortcut: Shortcut) -> Result<()>;

    /// Unregister by id. Unregistering an unknown id succeeds.
    async fn unregister(&self, id: &str) -> Result<()>;
}

/// Does nothing. For headless runs, CLI builds and tests.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopGlobalShortcutService;

#[async_trait]
impl GlobalShortcutService for NoopGlobalShortcutService {
    async fn register(&self, shortcut: Shortcut) -> Result<()> {
        tracing::debug!(id = %shortcut.id, accelerator = %shortcut.accelerator, "global shortcut — dropped (noop)");
        Ok(())
    }

    async fn unregister(&self, id: &str) -> Result<()> {
        tracing::debug!(id, "global shortcut unregistered — dropped (noop)");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_noop_service_accepts_registration_and_removal() {
        let shortcuts: &dyn GlobalShortcutService = &NoopGlobalShortcutService;

        shortcuts
            .register(Shortcut::new("quick-capture", "CmdOrCtrl+Shift+Space"))
            .await
            .unwrap();
        shortcuts.unregister("quick-capture").await.unwrap();
        shortcuts.unregister("never-registered").await.unwrap();
    }
}
