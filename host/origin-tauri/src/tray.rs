use crate::{HostConfig, focus_main_window};
use tauri::AppHandle;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

const MENU_SHOW: &str = "origin.show";
const MENU_QUIT: &str = "origin.quit";

pub(crate) fn install(app: &AppHandle, config: &HostConfig) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, MENU_SHOW, "Show window", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id("origin.tray")
        .tooltip(&config.tray_tooltip)
        .menu(&menu)
        // The menu is the tray's job; a left click should reveal the window instead.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_SHOW => focus_main_window(app),
            MENU_QUIT => app.exit(0),
            other => tracing::warn!(id = other, "unhandled tray menu item"),
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
