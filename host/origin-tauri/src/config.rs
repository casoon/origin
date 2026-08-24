/// Host-level product configuration.
///
/// In Phase 4 this is generated from the app manifest (ADR-0011); until then products
/// construct it by hand.
#[derive(Debug, Clone)]
pub struct HostConfig {
    /// Reverse-DNS product id, e.g. `dev.origin.demo`. Also scopes credentials in the
    /// system keychain, so two Origin apps cannot read each other's tokens.
    pub app_id: String,
    pub tray: bool,
    pub tray_tooltip: String,
    /// Closing the last window hides the app instead of quitting it.
    pub close_to_tray: bool,
    pub single_instance: bool,
    pub window_state: bool,
}

impl HostConfig {
    pub fn new(app_id: impl Into<String>) -> Self {
        let app_id = app_id.into();
        Self {
            tray_tooltip: app_id.clone(),
            app_id,
            tray: false,
            close_to_tray: false,
            single_instance: true,
            window_state: true,
        }
    }

    pub fn with_tray(mut self, tooltip: impl Into<String>) -> Self {
        self.tray = true;
        self.tray_tooltip = tooltip.into();
        self
    }

    pub fn with_close_to_tray(mut self, close_to_tray: bool) -> Self {
        self.close_to_tray = close_to_tray;
        self
    }
}
