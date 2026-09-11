# origin-workspace-watch

Filesystem watcher adapter for Origin workspace roots (`WorkspaceWatcher`, B4).

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_platform::{WorkspaceRoot, WorkspaceWatcher};
use origin_workspace_watch::NotifyWorkspaceWatcher;

let watcher = NotifyWorkspaceWatcher::new();
let root = WorkspaceRoot::new("/path/to/project")?;

let mut handle = watcher.watch(&root).await?;
tokio::spawn(async move {
    while let Ok(change) = handle.receiver.recv().await {
        println!("Changed: {:?}", change.path);
    }
});
```

Provides event-driven filesystem change monitoring over repository directories using the
`notify` crate (backed by OS facilities like FSEvents, inotify, and ReadDirectoryChangesW).
Runs independently of Tauri on background threads.

## Stability

Pre-1.0 (`0.2.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
