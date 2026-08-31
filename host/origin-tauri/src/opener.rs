use async_trait::async_trait;
use origin_domain::{AppError, Result};
use origin_platform::Opener;
use tauri::{AppHandle, Runtime};
use tauri_plugin_opener::OpenerExt;

/// Opens URLs in the user's default browser.
///
/// Deliberately limited to URLs: this is not a general shell escape, and a product
/// that needs to run local programs must define its own contract and capability
/// (ADR-0007).
#[derive(Debug, Clone)]
pub struct TauriOpener<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriOpener<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

#[async_trait]
impl<R: Runtime> Opener for TauriOpener<R> {
    async fn open_url(&self, url: &str) -> Result<()> {
        // Refuse anything that is not http(s) before it reaches the OS: `file://`
        // and custom schemes are how "open a link" turns into "launch a program".
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            return Err(AppError::validation(format!(
                "refusing to open {url:?}: only http and https URLs are allowed"
            )));
        }

        self.app
            .opener()
            .open_url(url, None::<&str>)
            .map_err(|error| AppError::internal(format!("cannot open url: {error}")))
    }
}
