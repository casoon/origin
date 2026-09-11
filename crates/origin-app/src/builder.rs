use crate::application::Application;
use crate::module::{ApplicationModule, ModuleRegistry};
use crate::platform::Platform;
use origin_accounts::{AccountService, AccountStore};
use origin_auth::TokenStore;
use origin_connector::{Connector, ConnectorRegistry};
use origin_domain::{AppError, Clock, SystemClock};
use origin_events::EventBus;
use origin_http::HttpClient;
use origin_jobs::Jobs;
use origin_platform::{
    ConfirmationService, DenyingConfirmationService, GlobalShortcutService, MemoryProcessRunner,
    MemoryWorkspaceFs, MemoryWorkspaceWatcher, NoopGlobalShortcutService, NoopNotificationService,
    NotificationService, Opener, ProcessAllowlist, ProcessRunner, TrayService, WorkspaceFs,
    WorkspaceWatcher,
};
use origin_secrets::{MemorySecretStore, SecretStore};
use origin_settings::{Settings, StorageSettingsStore};
use origin_storage::{Cache, MemoryStorage, Storage};
use origin_sync::SyncEngine;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// A required component was never provided. There is no implicit fallback for
    /// storage or credentials — silently defaulting those would ship an application
    /// that loses data or keeps tokens in process memory.
    #[error("no {component} configured — call `.{component}(...)` on the builder")]
    MissingComponent { component: &'static str },

    #[error("module `{module}` failed to register: {source}")]
    ModuleRegistration {
        module: &'static str,
        #[source]
        source: AppError,
    },
}

/// Assembles an [`Application`] from ports and modules.
///
/// Clock and event bus have exactly one sensible default and are pre-filled. Storage,
/// credentials and notifications must be chosen explicitly — or taken from
/// [`ApplicationBuilder::in_memory`] for tests.
#[derive(Debug)]
pub struct ApplicationBuilder {
    clock: Arc<dyn Clock>,
    events: EventBus,
    storage: Option<Arc<dyn Storage>>,
    secrets: Option<Arc<dyn SecretStore>>,
    notifications: Option<Arc<dyn NotificationService>>,
    opener: Option<Arc<dyn Opener>>,
    http: Option<Arc<dyn HttpClient>>,
    /// Human confirmation for operations that need it. Defaults to deny-all
    /// (fail-closed) so a product that never wires a real prompt is safe.
    confirmation: Arc<dyn ConfirmationService>,
    /// The system tray, present only when the product declared it.
    tray: Option<Arc<dyn TrayService>>,
    connectors: ConnectorRegistry,
    modules: Vec<Box<dyn ApplicationModule>>,
    workspace_fs: Option<Arc<dyn WorkspaceFs>>,
    workspace_watcher: Option<Arc<dyn WorkspaceWatcher>>,
    process_runner: Option<Arc<dyn ProcessRunner>>,
    global_shortcuts: Option<Arc<dyn GlobalShortcutService>>,
}

impl Default for ApplicationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationBuilder {
    pub fn new() -> Self {
        Self {
            clock: Arc::new(SystemClock),
            events: EventBus::new(),
            storage: None,
            secrets: None,
            notifications: None,
            opener: None,
            http: None,
            confirmation: Arc::new(DenyingConfirmationService),
            tray: None,
            connectors: ConnectorRegistry::new(),
            modules: Vec::new(),
            workspace_fs: None,
            workspace_watcher: None,
            process_runner: None,
            global_shortcuts: None,
        }
    }

    /// A fully in-memory application: no files, no keychain, no notifications.
    ///
    /// This is the configuration from ADR-0002 — it makes the whole application
    /// testable without starting a desktop session.
    pub fn in_memory() -> Self {
        Self::new()
            .storage(Arc::new(MemoryStorage::new()))
            .secret_store(Arc::new(MemorySecretStore::new()))
            .notifications(Arc::new(NoopNotificationService))
            .workspace_fs(Arc::new(MemoryWorkspaceFs::new()))
            .workspace_watcher(Arc::new(MemoryWorkspaceWatcher::new()))
            .process_runner(Arc::new(MemoryProcessRunner::success(
                ProcessAllowlist::default(),
            )))
            .global_shortcuts(Arc::new(NoopGlobalShortcutService))
    }

    pub fn clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Share an existing bus, e.g. one the host layer already subscribed to.
    pub fn event_bus(mut self, events: EventBus) -> Self {
        self.events = events;
        self
    }

    pub fn storage(mut self, storage: Arc<dyn Storage>) -> Self {
        self.storage = Some(storage);
        self
    }

    pub fn secret_store(mut self, secrets: Arc<dyn SecretStore>) -> Self {
        self.secrets = Some(secrets);
        self
    }

    pub fn notifications(mut self, notifications: Arc<dyn NotificationService>) -> Self {
        self.notifications = Some(notifications);
        self
    }

    /// Grant the ability to open external URLs. Omitted means the product does not
    /// have the capability at all, not that it is disabled at runtime.
    pub fn opener(mut self, opener: Arc<dyn Opener>) -> Self {
        self.opener = Some(opener);
        self
    }

    /// Override the human confirmation service. The default denies everything, so a
    /// product that grants MCP mutation *must* call this — the error message in the
    /// MCP tool response tells the product author why.
    pub fn confirmation(mut self, confirmation: Arc<dyn ConfirmationService>) -> Self {
        self.confirmation = confirmation;
        self
    }

    /// Give the application a system tray. Products without a tray never call this
    /// and get `None` — modules that need one see it through the platform.
    pub fn tray(mut self, tray: Arc<dyn TrayService>) -> Self {
        self.tray = Some(tray);
        self
    }

    /// Give the application an HTTP client.
    ///
    /// One client for the whole application: it owns the connection pool, and several
    /// would defeat keep-alive (ADR-0014).
    pub fn http_client(mut self, http: Arc<dyn HttpClient>) -> Self {
        self.http = Some(http);
        self
    }

    /// Register a connector.
    ///
    /// The set of external services a build can reach is fixed here, at compile time,
    /// and is therefore auditable (ADR-0006).
    pub fn connector(mut self, connector: impl Connector) -> Self {
        self.connectors.insert(Arc::new(connector));
        self
    }

    /// Give the application a workspace filesystem adapter (B2).
    pub fn workspace_fs(mut self, workspace_fs: Arc<dyn WorkspaceFs>) -> Self {
        self.workspace_fs = Some(workspace_fs);
        self
    }

    /// Give the application a workspace watcher adapter (B3).
    pub fn workspace_watcher(mut self, workspace_watcher: Arc<dyn WorkspaceWatcher>) -> Self {
        self.workspace_watcher = Some(workspace_watcher);
        self
    }

    /// Give the application a process runner adapter (B1).
    pub fn process_runner(mut self, process_runner: Arc<dyn ProcessRunner>) -> Self {
        self.process_runner = Some(process_runner);
        self
    }

    /// Give the application a global shortcut service adapter (B5).
    pub fn global_shortcuts(mut self, global_shortcuts: Arc<dyn GlobalShortcutService>) -> Self {
        self.global_shortcuts = Some(global_shortcuts);
        self
    }

    pub fn module(mut self, module: impl ApplicationModule) -> Self {
        self.modules.push(Box::new(module));
        self
    }

    pub fn build(self) -> Result<Application, BuildError> {
        let storage = self.storage.ok_or(BuildError::MissingComponent {
            component: "storage",
        })?;
        let secrets = self.secrets.ok_or(BuildError::MissingComponent {
            component: "secret_store",
        })?;
        let notifications = self.notifications.ok_or(BuildError::MissingComponent {
            component: "notifications",
        })?;

        let cache = Cache::new(storage.clone(), self.clock.clone());
        let settings = Settings::new(Arc::new(StorageSettingsStore::new(
            storage.clone(),
            self.clock.clone(),
        )));

        let accounts = AccountService::new(
            AccountStore::new(storage.clone(), self.clock.clone()),
            TokenStore::new(secrets.clone()),
            self.events.clone(),
            storage.clone(),
            self.clock.clone(),
        );

        let jobs = Jobs::new(self.events.clone(), self.clock.clone());
        let sync = SyncEngine::new(storage.clone(), self.clock.clone(), self.events.clone());

        let platform = Platform {
            clock: self.clock,
            events: self.events,
            jobs,
            sync,
            storage,
            cache,
            secrets,
            settings,
            notifications,
            confirmation: self.confirmation,
            tray: self.tray,
            accounts,
            connectors: self.connectors,
            opener: self.opener,
            http: self.http,
            workspace_fs: self.workspace_fs,
            workspace_watcher: self.workspace_watcher,
            process_runner: self.process_runner,
            global_shortcuts: self.global_shortcuts,
        };

        let mut registry = ModuleRegistry::new(platform.clone());
        for module in &self.modules {
            let id = module.id();
            tracing::debug!(module = id, "registering module");
            module
                .register(&mut registry)
                .map_err(|source| BuildError::ModuleRegistration { module: id, source })?;
            registry.record_module(id);
        }

        tracing::info!(modules = ?registry.module_ids(), "application built");
        Ok(Application::new(platform, registry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ModuleRegistry;
    use origin_domain::Result;

    #[derive(Debug)]
    struct Counter(u32);

    #[derive(Debug)]
    struct CountingModule;

    impl ApplicationModule for CountingModule {
        fn id(&self) -> &'static str {
            "counting"
        }

        fn register(&self, registry: &mut ModuleRegistry) -> Result<()> {
            registry.provide(Arc::new(Counter(7)));
            Ok(())
        }
    }

    #[derive(Debug)]
    struct FailingModule;

    impl ApplicationModule for FailingModule {
        fn id(&self) -> &'static str {
            "failing"
        }

        fn register(&self, _registry: &mut ModuleRegistry) -> Result<()> {
            Err(AppError::configuration("missing api key"))
        }
    }

    #[test]
    fn a_module_service_is_resolvable_by_type() {
        let app = ApplicationBuilder::in_memory()
            .module(CountingModule)
            .build()
            .unwrap();

        assert_eq!(app.modules(), &["counting"]);
        assert_eq!(app.require::<Counter>().unwrap().0, 7);
    }

    #[test]
    fn resolving_an_unregistered_service_names_the_type() {
        let app = ApplicationBuilder::in_memory().build().unwrap();

        let error = app.require::<Counter>().unwrap_err();
        assert!(error.to_string().contains("Counter"), "got: {error}");
    }

    #[test]
    fn missing_storage_is_a_build_error_not_a_silent_default() {
        let error = ApplicationBuilder::new().build().unwrap_err();
        assert!(matches!(
            error,
            BuildError::MissingComponent {
                component: "storage"
            }
        ));
    }

    #[test]
    fn a_build_without_an_http_client_says_what_is_missing() {
        let app = ApplicationBuilder::in_memory().build().unwrap();

        let error = app.platform().http().unwrap_err();

        assert_eq!(error.kind(), origin_domain::ErrorKind::Configuration);
        assert!(error.to_string().contains("http_client"), "got: {error}");
    }

    #[test]
    fn a_build_without_an_opener_reports_a_permission_error() {
        let app = ApplicationBuilder::in_memory().build().unwrap();

        // Capabilities are absent, not disabled: nothing can turn this on at runtime.
        assert_eq!(
            app.platform().opener().unwrap_err().kind(),
            origin_domain::ErrorKind::Permission
        );
    }

    #[test]
    fn in_memory_wires_workspace_and_process_and_shortcuts() {
        let app = ApplicationBuilder::in_memory().build().unwrap();

        assert!(app.platform().workspace_fs().is_ok());
        assert!(app.platform().workspace_watcher().is_ok());
        assert!(app.platform().process_runner().is_ok());
        assert!(app.platform().global_shortcuts().is_ok());
    }

    #[test]
    fn clean_build_reports_missing_workspace_components() {
        let app = ApplicationBuilder::new()
            .storage(Arc::new(MemoryStorage::new()))
            .secret_store(Arc::new(MemorySecretStore::new()))
            .notifications(Arc::new(NoopNotificationService))
            .build()
            .unwrap();

        assert!(app.platform().workspace_fs().is_err());
        assert!(app.platform().workspace_watcher().is_err());
        assert!(app.platform().process_runner().is_err());
        assert!(app.platform().global_shortcuts().is_err());
    }

    #[test]
    fn registered_connectors_are_resolvable_and_unknown_ones_are_not() {
        use origin_connector::{AuthKind, Connector, ConnectorDescriptor};
        use origin_domain::{AccountId, ConnectorId};

        #[derive(Debug)]
        struct TestConnector;

        #[async_trait::async_trait]
        impl Connector for TestConnector {
            fn id(&self) -> ConnectorId {
                ConnectorId::new("test")
            }

            fn descriptor(&self) -> ConnectorDescriptor {
                ConnectorDescriptor::new(self.id(), "Test", AuthKind::None)
            }

            async fn verify(
                &self,
                _account: &AccountId,
            ) -> Result<origin_connector::AccountIdentity> {
                unimplemented!("not needed for this test")
            }
        }

        let app = ApplicationBuilder::in_memory()
            .connector(TestConnector)
            .build()
            .unwrap();

        assert_eq!(
            app.platform().connectors.ids(),
            vec![ConnectorId::new("test")]
        );
        assert!(
            app.platform()
                .connectors
                .require(&ConnectorId::new("absent"))
                .is_err()
        );
    }

    #[test]
    fn a_failing_module_names_itself() {
        let error = ApplicationBuilder::in_memory()
            .module(FailingModule)
            .build()
            .unwrap_err();

        assert!(error.to_string().contains("failing"), "got: {error}");
    }
}
