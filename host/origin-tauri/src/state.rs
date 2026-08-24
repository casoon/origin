use crate::HostConfig;
use origin_app::Application;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// Tauri-managed state: the assembled application plus its host configuration.
#[derive(Debug)]
pub struct OriginState {
    application: Arc<Application>,
    config: HostConfig,
    /// Stops the sync scheduler. Held here so shutdown can end it deliberately rather
    /// than relying on process exit.
    scheduler: CancellationToken,
}

impl OriginState {
    pub(crate) fn new(
        application: Application,
        config: HostConfig,
        scheduler: CancellationToken,
    ) -> Self {
        Self {
            application: Arc::new(application),
            config,
            scheduler,
        }
    }

    /// Stop the background scheduler.
    pub fn shutdown(&self) {
        self.scheduler.cancel();
    }

    pub fn application(&self) -> Arc<Application> {
        self.application.clone()
    }

    pub fn config(&self) -> &HostConfig {
        &self.config
    }
}
