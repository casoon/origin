//! Memory double for [`WorkspaceWatcher`] — pushes synthetic changes for tests.
//!
//! Never use this in a shipped application.

use crate::watcher::{BUFFER_SIZE, WatchHandle, WorkspaceChange, WorkspaceWatcher};
use crate::workspace::WorkspaceRoot;
use async_trait::async_trait;
use origin_domain::Result;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::broadcast;

/// Records watcher registrations and allows tests to push synthetic change events.
#[derive(Debug, Default)]
pub struct MemoryWorkspaceWatcher {
    senders: Mutex<HashMap<WorkspaceRoot, broadcast::Sender<WorkspaceChange>>>,
}

impl MemoryWorkspaceWatcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a synthetic change into the watcher — as if the filesystem changed.
    /// Returns `true` if at least one subscriber received it.
    pub fn emit(&self, root: &WorkspaceRoot, change: WorkspaceChange) -> usize {
        let senders = self.senders.lock().expect("poisoned");
        match senders.get(root) {
            Some(sender) => sender.send(change).unwrap_or(0),
            None => 0,
        }
    }
}

#[async_trait]
impl WorkspaceWatcher for MemoryWorkspaceWatcher {
    async fn watch(&self, root: &WorkspaceRoot) -> Result<WatchHandle> {
        let mut senders = self.senders.lock().expect("poisoned");
        let sender = match senders.get(root) {
            Some(existing) => existing.clone(),
            None => {
                let (tx, _) = broadcast::channel(BUFFER_SIZE);
                senders.insert(root.clone(), tx.clone());
                tx
            }
        };

        Ok(WatchHandle::new(sender.subscribe()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn watching_twice_returns_different_handles_same_channel() {
        let watcher = MemoryWorkspaceWatcher::new();
        let root = WorkspaceRoot::new(std::env::temp_dir().join("repo")).unwrap();

        let mut handle_a = watcher.watch(&root).await.unwrap();
        let mut handle_b = watcher.watch(&root).await.unwrap();

        watcher.emit(&root, WorkspaceChange::modified("README.md"));

        assert_eq!(handle_a.recv().await.unwrap().path(), "README.md");
        assert_eq!(handle_b.recv().await.unwrap().path(), "README.md");
    }

    #[tokio::test]
    async fn different_roots_get_different_channels() {
        let watcher = MemoryWorkspaceWatcher::new();
        let root_a = WorkspaceRoot::new(std::env::temp_dir().join("a")).unwrap();
        let root_b = WorkspaceRoot::new(std::env::temp_dir().join("b")).unwrap();

        let mut handle_a = watcher.watch(&root_a).await.unwrap();
        let mut handle_b = watcher.watch(&root_b).await.unwrap();

        // Emit only to A
        watcher.emit(&root_a, WorkspaceChange::modified("a.txt"));
        // Emit only to B
        watcher.emit(&root_b, WorkspaceChange::modified("b.txt"));

        let a_event = handle_a.try_recv().unwrap();
        let b_event = handle_b.try_recv().unwrap();

        assert_eq!(a_event.path(), "a.txt");
        assert_eq!(b_event.path(), "b.txt");

        // A's channel should have nothing more
        assert!(handle_a.try_recv().is_err());
    }

    #[tokio::test]
    async fn emitting_to_an_unwatched_root_delivers_to_no_one() {
        let watcher = MemoryWorkspaceWatcher::new();
        let root = WorkspaceRoot::new(std::env::temp_dir().join("unwatched")).unwrap();

        let delivered = watcher.emit(&root, WorkspaceChange::created("nope.txt"));
        assert_eq!(delivered, 0);
    }
}
