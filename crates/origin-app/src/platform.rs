use origin_accounts::AccountService;
use origin_connector::ConnectorRegistry;
use origin_domain::{AppError, Clock, Result};
use origin_events::EventBus;
use origin_http::HttpClient;
use origin_jobs::Jobs;
use origin_platform::{
    ConfirmationService, GlobalShortcutService, NotificationService, Opener, ProcessRunner,
    TrayService, WorkspaceFs, WorkspaceWatcher,
};
use origin_secrets::SecretStore;
use origin_settings::Settings;
use origin_storage::{Cache, Storage};
use origin_sync::SyncEngine;
use std::sync::Arc;

/// The platform services every module may rely on.
///
/// Cloning is cheap and shares the same instances.
///
/// Optional fields are capabilities the product did not grant. They are `None` because
/// the composition root left them out, not because they are switched off at runtime —
/// a build that cannot reach the network is a build that cannot reach the network.
#[derive(Debug, Clone)]
pub struct Platform {
    pub clock: Arc<dyn Clock>,
    pub events: EventBus,
    pub storage: Arc<dyn Storage>,
    pub cache: Cache,
    pub secrets: Arc<dyn SecretStore>,
    pub settings: Settings,
    pub notifications: Arc<dyn NotificationService>,
    /// Human confirmation for mutating operations. The deny-all default keeps MCP
    /// safe by construction; only products with a real prompt override it.
    pub confirmation: Arc<dyn ConfirmationService>,
    /// The system tray handle, present only when the product declared it.
    pub tray: Option<Arc<dyn TrayService>>,
    /// Background jobs: progress, cancellation, uniform lifecycle.
    pub jobs: Jobs,
    /// Decides when registered sync targets run.
    pub sync: SyncEngine,
    /// Connected accounts across all connectors.
    pub accounts: AccountService,
    /// The connectors this build was compiled with.
    pub connectors: ConnectorRegistry,
    /// Present only when the product declared the capability to open external URLs.
    pub opener: Option<Arc<dyn Opener>>,
    /// Present only when the product talks to external services.
    pub http: Option<Arc<dyn HttpClient>>,
    /// Present only when the product grants workspace filesystem access (B2).
    pub workspace_fs: Option<Arc<dyn WorkspaceFs>>,
    /// Present only when the product watches workspace filesystem changes (B3).
    pub workspace_watcher: Option<Arc<dyn WorkspaceWatcher>>,
    /// Present only when the product allows executing external processes (B1).
    pub process_runner: Option<Arc<dyn ProcessRunner>>,
    /// Present only when the product registers global shortcuts (B5).
    pub global_shortcuts: Option<Arc<dyn GlobalShortcutService>>,
}

impl Platform {
    /// The HTTP client, or a configuration error naming what is missing.
    ///
    /// Modules call this instead of unwrapping the field, so a product that forgot to
    /// wire a client gets an actionable message rather than a panic.
    pub fn http(&self) -> Result<Arc<dyn HttpClient>> {
        self.http.clone().ok_or_else(|| {
            AppError::configuration(
                "this application has no http client — add `.http_client(...)` to its \
                 composition root",
            )
        })
    }

    /// The URL opener, or a permission error.
    pub fn opener(&self) -> Result<Arc<dyn Opener>> {
        self.opener.clone().ok_or_else(|| {
            AppError::Permission("this application cannot open external urls".to_owned())
        })
    }

    /// The workspace filesystem, or a configuration error naming what is missing.
    pub fn workspace_fs(&self) -> Result<Arc<dyn WorkspaceFs>> {
        self.workspace_fs.clone().ok_or_else(|| {
            AppError::configuration(
                "this application has no workspace filesystem — add `.workspace_fs(...)` to its \
                 composition root",
            )
        })
    }

    /// The workspace watcher, or a configuration error naming what is missing.
    pub fn workspace_watcher(&self) -> Result<Arc<dyn WorkspaceWatcher>> {
        self.workspace_watcher.clone().ok_or_else(|| {
            AppError::configuration(
                "this application has no workspace watcher — add `.workspace_watcher(...)` to its \
                 composition root",
            )
        })
    }

    /// The process runner, or a configuration error naming what is missing.
    pub fn process_runner(&self) -> Result<Arc<dyn ProcessRunner>> {
        self.process_runner.clone().ok_or_else(|| {
            AppError::configuration(
                "this application has no process runner — add `.process_runner(...)` to its \
                 composition root",
            )
        })
    }

    /// The global shortcut service, or a configuration error naming what is missing.
    pub fn global_shortcuts(&self) -> Result<Arc<dyn GlobalShortcutService>> {
        self.global_shortcuts.clone().ok_or_else(|| {
            AppError::configuration(
                "this application has no global shortcut service — add `.global_shortcuts(...)` to its \
                 composition root",
            )
        })
    }
}
