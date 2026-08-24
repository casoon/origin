//! Two permission levels that are never mixed (ADR-0007).
//!
//! A product permission says what the app may do *at an external service*.
//! A platform permission says what it may do *on this machine*.

use serde::{Deserialize, Serialize};

/// Rights at an external service. Products extend this via their own connectors;
/// these are the shapes every connector has in common.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ProductPermission {
    Read { scope: String },
    Write { scope: String },
}

impl ProductPermission {
    pub fn read(scope: impl Into<String>) -> Self {
        Self::Read {
            scope: scope.into(),
        }
    }

    pub fn write(scope: impl Into<String>) -> Self {
        Self::Write {
            scope: scope.into(),
        }
    }

    pub fn is_write(&self) -> bool {
        matches!(self, Self::Write { .. })
    }
}

/// Rights on the local machine. Each maps to a Tauri capability in the host layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PlatformPermission {
    Filesystem,
    Shell,
    Process,
    Notifications,
    CredentialStore,
    GlobalShortcut,
    Autostart,
}
