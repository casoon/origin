//! The Tauri host layer (ADR-0001).
//!
//! Everything Tauri-specific about an Origin application lives here: plugin
//! registration, tray, window handling, the IPC surface and the bridge that forwards
//! platform events to the webview.
//!
//! A product's `main.rs` stays small:
//!
//! ```ignore
//! fn main() {
//!     let config = HostConfig::new("dev.origin.demo");
//!     origin_tauri::builder(&config)
//!         .invoke_handler(origin_handler![my_product_command])
//!         .setup(move |app| {
//!             let application = my_product::build(app.handle())?;
//!             origin_tauri::attach(app.handle(), application, &config)?;
//!             Ok(())
//!         })
//!         .run(tauri::generate_context!())
//!         .expect("failed to start");
//! }
//! ```

mod bridge;
pub mod commands;
mod config;
pub mod defaults;
mod opener;
mod state;
mod tray;

pub use commands::CommandError;
pub use config::HostConfig;
pub use opener::TauriOpener;
pub use state::OriginState;

pub use origin_notifications_tauri::TauriNotificationService;

use origin_app::Application;
use tauri::{AppHandle, Manager, Wry};
use tokio_util::sync::CancellationToken;

/// A `tauri::Builder` with the Origin plugin set already registered.
///
/// Single-instance must be the first plugin, which is easy to get wrong by hand —
/// one more reason for products not to assemble this themselves.
pub fn builder(config: &HostConfig) -> tauri::Builder<Wry> {
    let mut builder = tauri::Builder::default();

    if config.single_instance {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            focus_main_window(app);
        }));
    }

    builder = builder
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init());

    if config.window_state {
        builder = builder.plugin(tauri_plugin_window_state::Builder::default().build());
    }

    builder
}

/// Hand the assembled application to the desktop shell.
///
/// Call this from `setup`, after the composition root has built the [`Application`].
pub fn attach(app: &AppHandle, application: Application, config: &HostConfig) -> tauri::Result<()> {
    let scheduler = CancellationToken::new();
    let state = OriginState::new(application, config.clone(), scheduler.clone());

    // Started here rather than in the product: `setup` runs outside a runtime context,
    // and knowing which executor to use is the host layer's job (ADR-0001).
    let engine = state.application().platform().sync.clone();
    tauri::async_runtime::spawn(async move { engine.run(scheduler).await });

    bridge::forward_platform_events(app, state.application());

    if config.tray {
        tray::install(app, config)?;
    }

    app.manage(state);
    tracing::info!(app_id = %config.app_id, "origin host attached");
    Ok(())
}

/// Show and focus the main window, creating nothing — if it was closed to tray it is
/// only hidden.
pub fn focus_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Builds a Tauri invoke handler containing Origin's own commands plus the product's.
///
/// Tauri allows exactly one invoke handler per application, so products must not call
/// `tauri::generate_handler!` themselves — they would drop the platform commands.
#[macro_export]
macro_rules! origin_handler {
    ($($command:path),* $(,)?) => {
        ::tauri::generate_handler![
            $crate::commands::origin_app_info,
            $crate::commands::origin_setting_get,
            $crate::commands::origin_setting_set,
            $crate::commands::origin_settings_customised,
            $crate::commands::origin_open_url,
            $crate::commands::origin_accounts,
            $crate::commands::origin_account_disconnect,
            $crate::commands::origin_connectors,
            $crate::commands::origin_jobs,
            $crate::commands::origin_job_cancel,
            $crate::commands::origin_sync_status,
            $crate::commands::origin_sync_now,
            $crate::commands::origin_health,
            $($command),*
        ]
    };
}
