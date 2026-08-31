use crate::Platform;
use origin_domain::{AppError, Result};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// A cohesive feature area — Inbox, Projects, Traffic, Health.
///
/// Modules are compile-time components. Origin has no dynamic plugin loading: a module
/// is code that was linked in, which keeps the dependency graph honest and the binary
/// auditable.
pub trait ApplicationModule: fmt::Debug + Send + Sync + 'static {
    /// Stable identifier, used in logs and in the app manifest.
    fn id(&self) -> &'static str;

    /// Wire the module up: read settings, provide services, subscribe to events.
    fn register(&self, registry: &mut ModuleRegistry) -> Result<()>;
}

/// What a module registers into during startup.
#[derive(Default)]
pub struct ModuleRegistry {
    platform: Option<Platform>,
    services: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    module_ids: Vec<&'static str>,
}

impl fmt::Debug for ModuleRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModuleRegistry")
            .field("modules", &self.module_ids)
            .field("services", &self.services.len())
            .finish()
    }
}

impl ModuleRegistry {
    pub(crate) fn new(platform: Platform) -> Self {
        Self {
            platform: Some(platform),
            services: HashMap::new(),
            module_ids: Vec::new(),
        }
    }

    /// Platform services available to every module.
    pub fn platform(&self) -> &Platform {
        self.platform
            .as_ref()
            .expect("registry is always constructed with a platform")
    }

    /// Publish a service so other modules and the host layer can resolve it by type.
    ///
    /// Registering the same type twice replaces the previous instance — the later
    /// module in the composition root wins, which is what an explicit override means.
    pub fn provide<T: Send + Sync + 'static>(&mut self, service: Arc<T>) {
        self.services.insert(TypeId::of::<T>(), Box::new(service));
    }

    pub fn service<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.services
            .get(&TypeId::of::<T>())
            .and_then(|entry| entry.downcast_ref::<Arc<T>>())
            .cloned()
    }

    /// Resolve a service or fail with a configuration error naming the missing type.
    pub fn require<T: Send + Sync + 'static>(&self) -> Result<Arc<T>> {
        self.service::<T>().ok_or_else(|| {
            AppError::configuration(format!(
                "no module provided the service `{}`",
                std::any::type_name::<T>()
            ))
        })
    }

    pub(crate) fn record_module(&mut self, id: &'static str) {
        self.module_ids.push(id);
    }

    pub(crate) fn module_ids(&self) -> &[&'static str] {
        &self.module_ids
    }
}
