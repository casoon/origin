//! Typed user settings.
//!
//! A setting is declared once as a [`Setting`] constant — key and default in one
//! place — and read through [`Settings`]. Nothing addresses settings by loose string.
//!
//! ```
//! # use origin_settings::Setting;
//! pub const THEME: Setting<String> = Setting::new("ui.theme", || String::from("system"));
//! ```

mod settings;
mod store;

pub use settings::{Setting, Settings};
pub use store::{SettingsStore, StorageSettingsStore};

/// Storage namespace used for settings records.
pub const SETTINGS_NAMESPACE: &str = "origin.settings";
