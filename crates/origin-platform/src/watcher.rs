//! Workspace watcher contract (B4 of the Gitbit platform requirements).
//!
//! Event-driven local sync: watches a workspace for file changes so a product like
//! Gitbit does not have to poll. Changes flow as typed events (ARCHITECTURE.md Rule 10).

use crate::workspace::WorkspaceRoot;
use async_trait::async_trait;
use origin_domain::Result;
use std::fmt::Debug;
use tokio::sync::broadcast;

/// How many change events to buffer while a subscriber catches up.
pub(crate) const BUFFER_SIZE: usize = 64;

/// A file system change within a watched workspace root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceChange {
    Created { path: String },
    Modified { path: String },
    Removed { path: String },
}

impl WorkspaceChange {
    pub fn created(path: impl Into<String>) -> Self {
        Self::Created { path: path.into() }
    }

    pub fn modified(path: impl Into<String>) -> Self {
        Self::Modified { path: path.into() }
    }

    pub fn removed(path: impl Into<String>) -> Self {
        Self::Removed { path: path.into() }
    }

    pub fn path(&self) -> &str {
        match self {
            Self::Created { path } | Self::Modified { path } | Self::Removed { path } => path,
        }
    }
}

/// A live watcher subscription. Dropping it stops receiving events.
///
/// Obtained from [`WorkspaceWatcher::watch`]. The adapter that implements
/// [`WorkspaceWatcher`] maps platform-level file-system events into
/// [`WorkspaceChange`] and publishes them to the application event bus.
pub struct WatchHandle {
    receiver: broadcast::Receiver<WorkspaceChange>,
}

impl Debug for WatchHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WatchHandle").finish_non_exhaustive()
    }
}

impl WatchHandle {
    /// Wrap a broadcast receiver. Used by [`WorkspaceWatcher`] implementations.
    ///
    /// Public so an adapter crate can construct the handle it returns; the shape is
    /// otherwise opaque on purpose.
    pub fn new(receiver: broadcast::Receiver<WorkspaceChange>) -> Self {
        Self { receiver }
    }

    /// Wait for the next change event.
    pub async fn recv(&mut self) -> Result<WorkspaceChange, broadcast::error::RecvError> {
        self.receiver.recv().await
    }

    /// Returns `Some` if a change is already waiting, `None` otherwise.
    pub fn try_recv(&mut self) -> Result<WorkspaceChange, broadcast::error::TryRecvError> {
        self.receiver.try_recv()
    }
}

/// Watches a workspace root for file changes.
///
/// Implementations publish [`WorkspaceChange`] events to a per-root broadcast
/// channel. The memory double (for tests) pushes synthetic changes; the Tauri
/// adapter wraps `notify`.
#[async_trait]
pub trait WorkspaceWatcher: Debug + Send + Sync + 'static {
    /// Start watching `root`. The returned [`WatchHandle`] yields change events.
    ///
    /// Calling `watch` on an already-watched root may return a new handle to the
    /// same channel — implementations may deduplicate internally.
    async fn watch(&self, root: &WorkspaceRoot) -> Result<WatchHandle>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_change_path_accessor() {
        let change = WorkspaceChange::modified("src/main.rs");
        assert_eq!(change.path(), "src/main.rs");
    }
}
