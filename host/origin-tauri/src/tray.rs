use crate::{HostConfig, focus_main_window};
use async_trait::async_trait;
use origin_domain::{AppError, Result};
use origin_events::{EventBus, PlatformEvent, TrayItemSelected};
use origin_platform::{TrayBadge, TrayMenuItem, TrayService};
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Runtime};

const MENU_SHOW: &str = "origin.show";
const MENU_QUIT: &str = "origin.quit";

pub(crate) fn install<R: Runtime>(
    app: &AppHandle<R>,
    config: &HostConfig,
    events: EventBus,
) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, MENU_SHOW, "Show window", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id("origin.tray")
        .tooltip(&config.tray_tooltip)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            let id = event.id().as_ref();
            match id {
                MENU_SHOW => focus_main_window(app),
                MENU_QUIT => app.exit(0),
                // A product-provided item: publish a typed event rather than call product
                // code from the host (ARCHITECTURE.md rules 9/10).
                product_item => {
                    let _ = events.publish(PlatformEvent::TrayItemSelected(TrayItemSelected {
                        id: product_item.to_owned(),
                    }));
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                focus_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    tracing::debug!("tray installed");
    Ok(())
}

/// A [`TrayService`] backed by a running Tauri tray icon.
///
/// Generic over the runtime so it is not welded to `Wry` — the mock runtime used in
/// tests satisfies the same bound, which is what makes the adapter testable (G11).
pub struct TauriTrayService<R: Runtime> {
    app: AppHandle<R>,
    /// The base tooltip, without a badge suffix.
    base_title: Mutex<String>,
    badge: Mutex<TrayBadge>,
}

impl<R: Runtime> std::fmt::Debug for TauriTrayService<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TauriTrayService").finish_non_exhaustive()
    }
}

impl<R: Runtime> TauriTrayService<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self {
            app,
            base_title: Mutex::new(String::new()),
            badge: Mutex::new(TrayBadge::None),
        }
    }

    fn update_tooltip(&self) {
        if let Some(tray) = self.app.tray_by_id("origin.tray") {
            let base = self.base_title.lock().unwrap();
            let badge = *self.badge.lock().unwrap();
            let full = badge_suffix(badge, &base);
            let _ = tray.set_tooltip(Some(full));
        }
    }
}

fn badge_suffix(badge: TrayBadge, base: &str) -> String {
    match badge {
        TrayBadge::None => base.to_owned(),
        TrayBadge::Attention => format!("{base} ●"),
        TrayBadge::Count(n) => format!("{base} ({n})"),
    }
}

#[async_trait]
impl<R: Runtime> TrayService for TauriTrayService<R> {
    async fn set_title(&self, title: &str) -> Result<()> {
        *self.base_title.lock().unwrap() = title.to_owned();
        self.update_tooltip();
        Ok(())
    }

    async fn set_badge(&self, badge: TrayBadge) -> Result<()> {
        *self.badge.lock().unwrap() = badge;
        self.update_tooltip();
        tracing::debug!(?badge, "tray badge updated");
        Ok(())
    }

    async fn set_menu(&self, items: Vec<TrayMenuItem>) -> Result<()> {
        let Some(tray) = self.app.tray_by_id("origin.tray") else {
            return Ok(());
        };

        let mut menu_items: Vec<MenuItem<R>> = Vec::new();
        for item in &items {
            let menu_item =
                MenuItem::with_id(&self.app, &item.id, &item.label, item.enabled, None::<&str>)
                    .map_err(|e| {
                        AppError::internal(format!("tray menu item `{}`: {e}", item.id))
                    })?;
            menu_items.push(menu_item);
        }

        let separator = PredefinedMenuItem::separator(&self.app)
            .map_err(|e| AppError::internal(format!("tray separator: {e}")))?;
        let quit = MenuItem::with_id(&self.app, MENU_QUIT, "Quit", true, None::<&str>)
            .map_err(|e| AppError::internal(format!("Quit item: {e}")))?;

        let refs: Vec<&dyn tauri::menu::IsMenuItem<R>> = {
            let mut collected: Vec<&dyn tauri::menu::IsMenuItem<R>> = Vec::new();
            for item in &menu_items {
                collected.push(item);
            }
            collected.push(&separator);
            collected.push(&quit);
            collected
        };

        let menu = Menu::with_items(&self.app, &refs)
            .map_err(|e| AppError::internal(format!("tray menu: {e}")))?;

        let _ = tray.set_menu(Some(menu));
        tracing::debug!(count = items.len(), "tray menu updated");
        Ok(())
    }
}

/// Host-wiring tests (G11).
///
/// A mock Tauri runtime is started, so the adapter runs against a real `AppHandle`.
/// There is no tray icon registered in the mock, which is exactly the graceful path
/// worth pinning: every call must be a no-op, never a panic. Gated off Windows for the
/// same upstream reason as `src/commands.rs`.
#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_tray_service_degrades_gracefully_without_a_registered_tray() {
        let app = tauri::test::mock_app();
        let tray = TauriTrayService::new(app.handle().clone());

        tray.set_title("Origin Demo").await.unwrap();
        tray.set_badge(TrayBadge::Count(3)).await.unwrap();
        tray.set_menu(vec![
            TrayMenuItem::new("show", "Show window"),
            TrayMenuItem::new("quit", "Quit"),
        ])
        .await
        .unwrap();
    }
}
