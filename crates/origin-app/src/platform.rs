use origin_accounts::AccountService;
use origin_connector::ConnectorRegistry;
use origin_core::{AppError, Clock, Result};
use origin_events::EventBus;
use origin_http::HttpClient;
use origin_jobs::Jobs;
use origin_platform::{NotificationService, Opener};
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
}
