//! Platform contracts (ADR-0001).

mod confirmation;
mod notifications;
mod opener;
pub mod paths;
pub mod process;
pub mod shortcut;
pub mod tray;
pub mod watcher;
pub mod workspace;

pub mod memory_process;
pub mod memory_watcher;
pub mod memory_workspace;

#[cfg(feature = "testing")]
pub mod process_contract;

#[cfg(feature = "testing")]
pub mod watcher_contract;

#[cfg(feature = "testing")]
pub mod testing;

#[cfg(feature = "testing")]
pub mod workspace_contract;

pub use confirmation::{
    ConfirmationDecision, ConfirmationRequest, ConfirmationService, DenyingConfirmationService,
};
pub use notifications::{
    NoopNotificationService, Notification, NotificationAction, NotificationService, Urgency,
};
pub use opener::Opener;
pub use process::{ProcessAllowlist, ProcessOutput, ProcessRunner};
pub use shortcut::{GlobalShortcutService, NoopGlobalShortcutService, Shortcut};
pub use tray::{NoopTrayService, TrayBadge, TrayMenuItem, TrayService};
pub use watcher::{WatchHandle, WorkspaceChange, WorkspaceWatcher};
pub use workspace::{DirEntry, RelPath, WorkspaceFs, WorkspaceRoot};

pub use memory_process::MemoryProcessRunner;
pub use memory_watcher::MemoryWorkspaceWatcher;
pub use memory_workspace::MemoryWorkspaceFs;
