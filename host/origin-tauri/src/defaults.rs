//! The standard desktop wiring.
//!
//! Products call these from their composition root instead of repeating the same four
//! adapter constructions — and override any single one where they need something else
//! (ADR-0004, "convention by default, explicit override when needed").

use crate::{HostConfig, TauriNotificationService, TauriOpener};
use origin_core::Result;
use origin_http::HttpClient;
use origin_http_reqwest::ReqwestHttpClient;
use origin_platform::{NotificationService, Opener};
use origin_secrets::SecretStore;
use origin_secrets_system::SystemSecretStore;
use origin_storage::Storage;
use origin_storage_sqlite::SqliteStorage;
use std::sync::Arc;
use tauri::{AppHandle, Runtime};

/// Name of the database file inside the app data directory.
const DATABASE_FILE: &str = "origin.sqlite3";

/// SQLite storage in the platform's app-data directory.
///
/// The directory comes from `origin_platform::paths`, not from Tauri's path resolver:
/// a headless run of the same product has no `AppHandle` and must reach the same
/// database. Two independent derivations of one directory is how a headless mode ends
/// up looking at an empty file.
///
/// The file holds cache, read models and settings only — losing it costs a resync,
/// nothing more (ADR-0008).
pub fn storage<R: Runtime>(app: &AppHandle<R>, config: &HostConfig) -> Result<Arc<dyn Storage>> {
    let _ = app;

    let path = origin_platform::paths::data_dir(&config.app_id)?.join(DATABASE_FILE);
    tracing::debug!(path = %path.display(), "opening application database");

    Ok(Arc::new(SqliteStorage::open(path)?))
}

/// Credentials in the operating system credential store, scoped to this product.
pub fn secret_store(config: &HostConfig) -> Arc<dyn SecretStore> {
    Arc::new(SystemSecretStore::new(config.app_id.clone()))
}

/// Native notifications.
pub fn notifications<R: Runtime>(app: &AppHandle<R>) -> Arc<dyn NotificationService> {
    Arc::new(TauriNotificationService::new(app.clone()))
}

/// One HTTP client for the whole application.
///
/// The user agent identifies the product and version, which several APIs require and
/// most of them use when they need to contact an integrator about traffic.
pub fn http_client<R: Runtime>(
    app: &AppHandle<R>,
    config: &HostConfig,
) -> Result<Arc<dyn HttpClient>> {
    let package = app.package_info();
    let user_agent = format!("{}/{} ({})", package.name, package.version, config.app_id);

    Ok(Arc::new(ReqwestHttpClient::new(user_agent)?))
}

/// Opening external http(s) URLs in the user's browser.
pub fn opener<R: Runtime>(app: &AppHandle<R>) -> Arc<dyn Opener> {
    Arc::new(TauriOpener::new(app.clone()))
}
