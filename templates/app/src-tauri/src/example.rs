//! An example feature area. Delete it once you have a real one.
//!
//! Note what it does not do: it never opens a database, never touches Tauri, and never
//! shows a notification itself. It asks the platform. That is what keeps it testable
//! without a desktop session.

use origin_app::{ApplicationModule, ModuleRegistry, Platform};
use origin_domain::Result;
use origin_settings::Setting;
use std::sync::Arc;

/// A setting: key and default declared together.
const GREETING: Setting<String> = Setting::new("example.greeting", || "Hello".to_owned());

#[derive(Debug)]
pub struct ExampleService {
    platform: Platform,
}

impl ExampleService {
    fn new(platform: Platform) -> Self {
        Self { platform }
    }

    pub async fn greeting(&self) -> Result<String> {
        Ok(self.platform.settings.get(&GREETING).await?)
    }
}

#[derive(Debug)]
pub struct ExampleModule;

impl ApplicationModule for ExampleModule {
    fn id(&self) -> &'static str {
        "example"
    }

    fn register(&self, registry: &mut ModuleRegistry) -> Result<()> {
        registry.provide(Arc::new(ExampleService::new(registry.platform().clone())));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use origin_app::ApplicationBuilder;

    /// The quality gate from ADR-0002: the feature is exercised without starting Tauri.
    #[tokio::test]
    async fn the_greeting_comes_from_settings() {
        let application = ApplicationBuilder::in_memory()
            .module(ExampleModule)
            .build()
            .expect("build application");
        let service = application.require::<ExampleService>().unwrap();

        assert_eq!(service.greeting().await.unwrap(), "Hello");

        application
            .platform()
            .settings
            .set(&GREETING, &"Guten Tag".to_owned())
            .await
            .unwrap();

        assert_eq!(service.greeting().await.unwrap(), "Guten Tag");
    }
}
