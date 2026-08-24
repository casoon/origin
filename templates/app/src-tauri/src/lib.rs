//! __PRODUCT_NAME__

pub mod commands;
pub mod example;

use example::ExampleModule;
use origin_app::{Application, ApplicationBuilder};
use origin_tauri::{HostConfig, defaults, origin_handler};
use origin_telemetry::{Format, TelemetryConfig};
use tauri::AppHandle;

const APP_ID: &str = "__PRODUCT_ID__";

pub fn run() {
    origin_telemetry::init(TelemetryConfig {
        default_filter: "info,origin=debug,__CRATE_NAME_SNAKE__=debug".to_owned(),
        format: Format::Pretty,
        log_span_events: false,
        to_stderr: false,
    });

    let config = HostConfig::new(APP_ID);
    let setup_config = config.clone();

    origin_tauri::builder(&config)
        .invoke_handler(origin_handler![commands::example_greeting])
        .setup(move |app| {
            let application = build(app.handle(), &setup_config)?;
            origin_tauri::attach(app.handle(), application, &setup_config)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("__PRODUCT_NAME__ failed to start");
}

/// The composition root (ADR-0004).
///
/// This function is the architecture of the product: every dependency it has is
/// visible here. Add a `.connector(...)` when you integrate a service, a `.module(...)`
/// for each feature area.
fn build(app: &AppHandle, config: &HostConfig) -> origin_core::Result<Application> {
    ApplicationBuilder::new()
        .storage(defaults::storage(app, config)?)
        .secret_store(defaults::secret_store(config))
        .notifications(defaults::notifications(app))
        .module(ExampleModule)
        .build()
        .map_err(|error| origin_core::AppError::configuration(error.to_string()))
}
