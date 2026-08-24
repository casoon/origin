//! Platform contracts (ADR-0001).
//!
//! These traits are the only way domain code touches the operating system. Tauri-backed
//! implementations live in `adapters/` and `host/`; this crate must never depend on them.

mod notifications;
mod opener;
pub mod paths;

#[cfg(feature = "testing")]
pub mod testing;

pub use notifications::{NoopNotificationService, Notification, NotificationService, Urgency};
pub use opener::Opener;
