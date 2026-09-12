//! The workspace watcher contract (`WorkspaceWatcher`) over `notify` (B4).
//!
//! Event-driven local sync: a product like Gitbit does not have to poll a working
//! tree. `notify` runs the platform mechanism (FSEvents, inotify, ReadDirectoryChanges)
//! on its own thread and calls back; the callback converts each event to a
//! [`WorkspaceChange`] and pushes it onto a broadcast channel, which the returned
//! [`WatchHandle`] reads. Nothing here touches Tauri.

use async_trait::async_trait;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use origin_domain::{AppError, Result};
use origin_platform::{WatchHandle, WorkspaceChange, WorkspaceRoot, WorkspaceWatcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::sync::broadcast;

/// How many change events to buffer while a subscriber catches up.
const BUFFER: usize = 64;

/// Watches workspace roots with `notify`.
///
/// One `notify` watcher is kept per root for the lifetime of the adapter — dropping it
/// stops the platform watch. Calling `watch` twice for the same root reuses the
/// existing watcher and returns a fresh handle to the same channel.
#[derive(Debug, Default)]
pub struct NotifyWorkspaceWatcher {
    watchers: Mutex<HashMap<WorkspaceRoot, RecommendedWatcher>>,
}

impl NotifyWorkspaceWatcher {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl WorkspaceWatcher for NotifyWorkspaceWatcher {
    async fn watch(&self, root: &WorkspaceRoot) -> Result<WatchHandle> {
        let (sender, _) = broadcast::channel(BUFFER);

        {
            let watchers = self.watchers.lock().expect("watcher map poisoned");
            if watchers.contains_key(root) {
                return Ok(WatchHandle::new(sender.subscribe()));
            }
        }

        // The platform may report paths through the resolved root rather than the one
        // given (FSEvents: `/private/var/…` for `/var/…`), so relativise against both.
        let given = root.as_path().to_path_buf();
        let resolved = std::fs::canonicalize(&given).unwrap_or_else(|_| given.clone());
        let roots = [given, resolved];
        let forward = sender.clone();
        let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
            match event {
                Ok(event) => {
                    if let Some(change) = change_for(&event, &roots) {
                        // `send` fails only when nobody is subscribed, which is fine.
                        let _ = forward.send(change);
                    }
                }
                Err(error) => tracing::warn!(%error, "workspace watcher reported an error"),
            }
        })
        .map_err(|error| {
            AppError::internal(format!("cannot create a filesystem watcher: {error}"))
        })?;

        watcher
            .watch(root.as_path(), RecursiveMode::Recursive)
            .map_err(|error| {
                AppError::storage(format!(
                    "cannot watch {}: {error}",
                    root.as_path().display()
                ))
            })?;

        self.watchers
            .lock()
            .expect("watcher map poisoned")
            .insert(root.clone(), watcher);

        tracing::debug!(root = %root.as_path().display(), "workspace watch started");
        Ok(WatchHandle::new(sender.subscribe()))
    }
}

/// Translate a `notify` event into a workspace change, relative to the root.
///
/// `roots` are the spellings of the same root the platform may report paths under.
fn change_for(event: &Event, roots: &[PathBuf]) -> Option<WorkspaceChange> {
    let path = event.paths.first()?;
    let relative = roots
        .iter()
        .find_map(|root| path.strip_prefix(root).ok())
        .unwrap_or(path);

    // The root itself is not a change inside the workspace. FSEvents replays the
    // root's own creation when the watch starts right after it was created.
    if relative.as_os_str().is_empty() {
        return None;
    }

    // A POSIX-style relative path, so a change reads the same on every platform.
    let relative = relative.to_string_lossy().replace('\\', "/");

    match event.kind {
        EventKind::Create(_) => Some(WorkspaceChange::created(relative)),
        EventKind::Modify(_) => Some(WorkspaceChange::modified(relative)),
        EventKind::Remove(_) => Some(WorkspaceChange::removed(relative)),
        // Access events and metadata-only kinds are noise for a working-tree sync.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn only_write_kinds_map_to_a_change() {
        let root = [PathBuf::from("/repo")];

        let create = Event {
            kind: EventKind::Create(notify::event::CreateKind::File),
            paths: vec![PathBuf::from("/repo/src/new.rs")],
            attrs: Default::default(),
        };
        assert_eq!(
            change_for(&create, &root),
            Some(WorkspaceChange::created("src/new.rs"))
        );

        let access = Event {
            kind: EventKind::Access(notify::event::AccessKind::Read),
            paths: vec![PathBuf::from("/repo/src/main.rs")],
            attrs: Default::default(),
        };
        assert_eq!(change_for(&access, &root), None);
    }

    #[test]
    fn paths_under_the_resolved_root_are_relative_and_the_root_itself_is_ignored() {
        let roots = [PathBuf::from("/var/ws"), PathBuf::from("/private/var/ws")];

        let create = Event {
            kind: EventKind::Create(notify::event::CreateKind::File),
            paths: vec![PathBuf::from("/private/var/ws/new.txt")],
            attrs: Default::default(),
        };
        assert_eq!(
            change_for(&create, &roots),
            Some(WorkspaceChange::created("new.txt"))
        );

        let root_created = Event {
            kind: EventKind::Create(notify::event::CreateKind::Folder),
            paths: vec![PathBuf::from("/private/var/ws")],
            attrs: Default::default(),
        };
        assert_eq!(change_for(&root_created, &roots), None);
    }

    #[tokio::test]
    async fn a_file_created_inside_the_root_is_reported() {
        let dir =
            std::env::temp_dir().join(format!("origin-workspace-watch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let root = WorkspaceRoot::new(dir.clone()).expect("absolute root");
        let watcher = NotifyWorkspaceWatcher::new();
        let mut handle = watcher.watch(&root).await.expect("watch");

        // Let the platform watcher register before the change is made.
        tokio::time::sleep(Duration::from_millis(300)).await;
        std::fs::write(dir.join("new.txt"), b"hello").unwrap();

        let change = tokio::time::timeout(Duration::from_secs(10), handle.recv())
            .await
            .expect("a change must arrive within the timeout")
            .expect("the channel is open");

        assert!(
            change.path().contains("new.txt"),
            "unexpected change: {change:?}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
