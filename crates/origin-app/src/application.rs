use crate::module::ModuleRegistry;
use crate::platform::Platform;
use origin_domain::Result;
use serde::Serialize;
use std::sync::Arc;

/// Product identity, as the frontend needs it to render the shell.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    /// Modules compiled into this build, in registration order.
    pub modules: Vec<String>,
}

/// A fully assembled application.
///
/// It knows nothing about Tauri. The host layer takes one of these and exposes it to
/// the desktop shell; a CLI or a headless agent could take the same value.
#[derive(Debug)]
pub struct Application {
    platform: Platform,
    registry: ModuleRegistry,
}

impl Application {
    pub(crate) fn new(platform: Platform, registry: ModuleRegistry) -> Self {
        Self { platform, registry }
    }

    pub fn platform(&self) -> &Platform {
        &self.platform
    }

    /// Resolve a service registered by a module.
    pub fn service<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.registry.service::<T>()
    }

    /// Resolve a service or fail with a configuration error.
    pub fn require<T: Send + Sync + 'static>(&self) -> Result<Arc<T>> {
        self.registry.require::<T>()
    }

    /// Ids of the registered modules, in registration order.
    pub fn modules(&self) -> &[&'static str] {
        self.registry.module_ids()
    }
}
