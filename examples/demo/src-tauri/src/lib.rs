//! Origin Demo — the reference application.
//!
//! It is deliberately small. Its job is to show the shape of an Origin application:
//! a composition root, one module, a background loop, and a frontend that talks to
//! none of it directly.

pub mod commands;
pub mod connector;
pub mod mcp;
pub mod pulse;

use connector::DemoConnector;
use origin_app::{Application, ApplicationBuilder};
use origin_platform::NoopNotificationService;
use origin_secrets::MemorySecretStore;
use origin_storage_sqlite::SqliteStorage;
use origin_tauri::{HostConfig, defaults, origin_handler};
use origin_telemetry::{Format, TelemetryConfig};
use pulse::PulseModule;
use std::sync::Arc;
use tauri::AppHandle;

const APP_ID: &str = "dev.origin.demo";

pub fn run() {
    origin_telemetry::init(TelemetryConfig {
        default_filter: "info,origin=debug,origin_demo=debug".to_owned(),
        format: Format::Pretty,
        log_span_events: false,
        to_stderr: false,
    });

    let config = HostConfig::new(APP_ID).with_tray("Origin Demo");
    let setup_config = config.clone();

    origin_tauri::builder(&config)
        .invoke_handler(origin_handler![
            commands::demo_snapshot,
            commands::demo_refresh,
        ])
        .setup(move |app| {
            let application = build(app.handle(), &setup_config)?;

            // `attach` also starts the sync scheduler, so the tray and notifications
            // keep reporting while the window is closed.
            origin_tauri::attach(app.handle(), application, &setup_config)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("origin demo failed to start");
}

/// The composition root (ADR-0004).
///
/// This function *is* the architecture of the product: every dependency the demo has
/// is visible in these few lines.
fn build(app: &AppHandle, config: &HostConfig) -> origin_core::Result<Application> {
    let application = ApplicationBuilder::new()
        .storage(defaults::storage(app, config)?)
        .secret_store(defaults::secret_store(config))
        .notifications(defaults::notifications(app))
        .opener(defaults::opener(app))
        .http_client(defaults::http_client(app, config)?)
        .connector(DemoConnector)
        .module(PulseModule)
        .build()
        .map_err(|error| origin_core::AppError::configuration(error.to_string()))?;

    Ok(application)
}

/// Serve MCP on stdio, with no window and no Tauri.
///
/// This is the architecture test from §52/§53 of the concept, executed: the same core
/// that drives the desktop shell also answers an external AI client, and neither knows
/// about the other.
pub fn run_mcp() -> origin_core::Result<()> {
    // stdout carries the protocol. A single log line there corrupts the stream, and the
    // client reports a parse error that points nowhere near logging.
    origin_telemetry::init(TelemetryConfig {
        default_filter: "warn,origin_mcp=info".to_owned(),
        ..TelemetryConfig::for_stdout_protocol()
    });

    let runtime = tokio::runtime::Runtime::new().map_err(|error| {
        origin_core::AppError::internal(format!("cannot start runtime: {error}"))
    })?;

    runtime.block_on(async {
        let application = build_headless()?;
        let server = mcp::server(&application)?;

        origin_mcp_stdio::serve(&server).await
    })
}

/// The headless composition root.
///
/// Same modules and the same database file as the desktop build — the directory comes
/// from `origin_platform::paths`, which is exactly why both agree. What differs is
/// only what has no meaning without a window: notifications, the URL opener, the tray.
///
/// Credentials use an in-memory store here: a headless process started by an AI client
/// must not raise a keychain prompt nobody is present to answer. A product that needs
/// real credentials headless has to solve that deliberately.
fn build_headless() -> origin_core::Result<Application> {
    let path = origin_platform::paths::data_dir(APP_ID)?.join("origin.sqlite3");

    ApplicationBuilder::new()
        .storage(Arc::new(SqliteStorage::open(path)?))
        .secret_store(Arc::new(MemorySecretStore::new()))
        .notifications(Arc::new(NoopNotificationService))
        .connector(DemoConnector)
        .module(PulseModule)
        .build()
        .map_err(|error| origin_core::AppError::configuration(error.to_string()))
}
