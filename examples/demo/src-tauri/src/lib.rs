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
use origin_mcp::McpServer;
use origin_mcp_http::{Discovery, HttpTransport, Token, is_alive, proxy_streams};
use origin_platform::{NoopNotificationService, TrayBadge, TrayService};
use origin_secrets::MemorySecretStore;
use origin_storage_sqlite::SqliteStorage;
use origin_tauri::{HostConfig, TauriTrayService, defaults, origin_handler};
use origin_telemetry::{Format, TelemetryConfig};
use pulse::PulseModule;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::io::BufReader;
use tokio_util::sync::CancellationToken;

const APP_ID: &str = "dev.origin.demo";

/// Where the running GUI publishes its MCP endpoint (G16), read by a headless start
/// (G17).
const MCP_DISCOVERY_FILE: &str = "mcp-http.json";

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

            // Build the MCP server before `attach` consumes the application, so the
            // running GUI can serve it over HTTP (G16).
            let server = mcp::server(&application)?;
            let tray = application.platform().tray.clone();

            // `attach` also starts the sync scheduler, so the tray and notifications
            // keep reporting while the window is closed.
            origin_tauri::attach(app.handle(), application, &setup_config)?;

            start_mcp_http(app.handle(), server, tray)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("origin demo failed to start");
}

/// The composition root (ADR-0004).
///
/// This function *is* the architecture of the product: every dependency the demo has
/// is visible in these few lines.
fn build(app: &AppHandle, config: &HostConfig) -> origin_domain::Result<Application> {
    let application = ApplicationBuilder::new()
        .storage(defaults::storage(app, config)?)
        .secret_store(defaults::secret_store(config))
        .notifications(defaults::notifications(app))
        .opener(defaults::opener(app))
        .http_client(defaults::http_client(app, config)?)
        .tray(Arc::new(TauriTrayService::new(app.clone())))
        .connector(DemoConnector)
        .module(PulseModule)
        .build()
        .map_err(|error| origin_domain::AppError::configuration(error.to_string()))?;

    Ok(application)
}

/// Publish the MCP endpoint over loopback while the GUI runs (G16), with a negotiated
/// token (G19), and show the tray indicator only while a session is active (G18).
fn start_mcp_http(
    app: &AppHandle,
    server: McpServer,
    tray: Option<Arc<dyn TrayService>>,
) -> origin_domain::Result<()> {
    let path = origin_platform::paths::data_dir(APP_ID)?.join(MCP_DISCOVERY_FILE);
    let token = Token::generate();

    let transport = tauri::async_runtime::block_on(HttpTransport::bind(Some(token)))
        .map_err(|error| origin_domain::AppError::internal(error.to_string()))?;

    transport
        .discovery()
        .write(&path)
        .map_err(|error| origin_domain::AppError::internal(error.to_string()))?;

    let url = transport.url();
    let activity = transport.activity();
    tracing::info!(%url, "mcp http endpoint published");

    let stop = CancellationToken::new();

    // G18: a tray indicator while — and only while — an external AI is connected.
    if let Some(tray) = tray {
        let stop_monitor = stop.clone();
        tauri::async_runtime::spawn(async move {
            const IDLE: std::time::Duration = std::time::Duration::from_secs(60);
            const POLL: std::time::Duration = std::time::Duration::from_secs(5);

            let mut shown = false;
            loop {
                tokio::select! {
                    _ = stop_monitor.cancelled() => break,
                    _ = tokio::time::sleep(POLL) => {}
                }

                let active = activity.is_active(IDLE);
                if active != shown {
                    let badge = if active {
                        TrayBadge::Attention
                    } else {
                        TrayBadge::None
                    };
                    let _ = tray.set_badge(badge).await;
                    shown = active;
                    tracing::debug!(active, "mcp session indicator updated");
                }
            }
        });
    }

    tauri::async_runtime::spawn(async move {
        if let Err(error) = transport.serve(Arc::new(server), stop).await {
            tracing::warn!(%error, "mcp http endpoint stopped");
        }
        // The endpoint is gone with the process; do not leave a stale pointer behind.
        Discovery::remove(&path);
    });

    let _ = app;
    Ok(())
}

/// Serve MCP on stdio, with no window and no Tauri.
///
/// This is the architecture test from §52/§53 of the concept, executed: the same core
/// that drives the desktop shell also answers an external AI client, and neither knows
/// about the other.
///
/// If a GUI is already running, this process does **not** open a second database
/// connection (G17): it finds the published HTTP endpoint and proxies its stdio to it.
pub fn run_mcp() -> origin_domain::Result<()> {
    // stdout carries the protocol. A single log line there corrupts the stream, and the
    // client reports a parse error that points nowhere near logging.
    origin_telemetry::init(TelemetryConfig {
        default_filter: "warn,origin_mcp=info".to_owned(),
        ..TelemetryConfig::for_stdout_protocol()
    });

    let runtime = tokio::runtime::Runtime::new().map_err(|error| {
        origin_domain::AppError::internal(format!("cannot start runtime: {error}"))
    })?;

    runtime.block_on(async {
        let path = origin_platform::paths::data_dir(APP_ID)?.join(MCP_DISCOVERY_FILE);

        if let Some(discovery) = Discovery::read(&path) {
            if is_alive(&discovery.url, Some(&discovery.token)).await {
                let input = BufReader::new(tokio::io::stdin());
                let output = tokio::io::stdout();
                return proxy_streams(input, output, &discovery.url, Some(&discovery.token)).await;
            }

            tracing::debug!(
                "mcp discovery file present but its endpoint is not reachable; \
                 starting a stdio instance instead"
            );
        }

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
fn build_headless() -> origin_domain::Result<Application> {
    let path = origin_platform::paths::data_dir(APP_ID)?.join("origin.sqlite3");

    ApplicationBuilder::new()
        .storage(Arc::new(SqliteStorage::open(path)?))
        .secret_store(Arc::new(MemorySecretStore::new()))
        .notifications(Arc::new(NoopNotificationService))
        .connector(DemoConnector)
        .module(PulseModule)
        .build()
        .map_err(|error| origin_domain::AppError::configuration(error.to_string()))
}
