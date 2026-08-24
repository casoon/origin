use crate::store::SettingsStore;
use origin_core::{AppError, Result};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use std::sync::Arc;

/// Declaration of one setting: its key and its default, together.
#[derive(Debug)]
pub struct Setting<T> {
    key: &'static str,
    default: fn() -> T,
    _type: PhantomData<fn() -> T>,
}

impl<T> Setting<T> {
    pub const fn new(key: &'static str, default: fn() -> T) -> Self {
        Self {
            key,
            default,
            _type: PhantomData,
        }
    }

    pub const fn key(&self) -> &'static str {
        self.key
    }

    pub fn default_value(&self) -> T {
        (self.default)()
    }
}

/// Typed access to the settings store.
#[derive(Debug, Clone)]
pub struct Settings {
    store: Arc<dyn SettingsStore>,
}

impl Settings {
    pub fn new(store: Arc<dyn SettingsStore>) -> Self {
        Self { store }
    }

    /// The stored value, or the declared default.
    ///
    /// A stored value that no longer decodes — because the setting's type changed
    /// between releases — falls back to the default and logs a warning. Refusing to
    /// start over a stale preference would be worse than ignoring it.
    pub async fn get<T: DeserializeOwned>(&self, setting: &Setting<T>) -> Result<T> {
        let Some(raw) = self.store.get_raw(setting.key()).await? else {
            return Ok(setting.default_value());
        };

        match serde_json::from_str(&raw) {
            Ok(value) => Ok(value),
            Err(error) => {
                tracing::warn!(
                    setting = setting.key(),
                    %error,
                    "stored setting could not be decoded, falling back to default"
                );
                Ok(setting.default_value())
            }
        }
    }

    pub async fn set<T: Serialize>(&self, setting: &Setting<T>, value: &T) -> Result<()> {
        let encoded = serde_json::to_string(value).map_err(|error| {
            AppError::validation(format!("cannot encode setting {}: {error}", setting.key()))
        })?;
        self.store.set_raw(setting.key(), encoded).await
    }

    /// Drop the stored value so the next read returns the default.
    pub async fn reset<T>(&self, setting: &Setting<T>) -> Result<()> {
        self.store.remove(setting.key()).await
    }

    /// Untyped read for generic settings UIs, which do not know the concrete types.
    ///
    /// Prefer [`Settings::get`] wherever the setting is known at compile time.
    pub async fn get_json(&self, key: &str) -> Result<Option<serde_json::Value>> {
        let Some(raw) = self.store.get_raw(key).await? else {
            return Ok(None);
        };
        serde_json::from_str(&raw)
            .map(Some)
            .map_err(|error| AppError::storage(format!("cannot decode setting {key}: {error}")))
    }

    /// Untyped write for generic settings UIs.
    ///
    /// The value is not validated against the setting's declared type — a generic
    /// caller has no way to know it. A value that no longer decodes is ignored on read
    /// (see [`Settings::get`]), so a bad write degrades to the default rather than
    /// breaking startup.
    pub async fn set_json(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        let encoded = serde_json::to_string(value).map_err(|error| {
            AppError::validation(format!("cannot encode setting {key}: {error}"))
        })?;
        self.store.set_raw(key, encoded).await
    }

    /// Keys that currently have a stored value. Settings left at their default are
    /// not listed.
    pub async fn customised_keys(&self) -> Result<Vec<String>> {
        self.store.keys().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageSettingsStore;
    use origin_core::testing::FakeClock;
    use origin_storage::MemoryStorage;
    use time::macros::datetime;

    const REFRESH_MINUTES: Setting<u32> = Setting::new("sync.refresh_minutes", || 5);

    fn settings() -> Settings {
        let clock = Arc::new(FakeClock::new(datetime!(2026-08-23 10:00 UTC)));
        let store = StorageSettingsStore::new(Arc::new(MemoryStorage::new()), clock);
        Settings::new(Arc::new(store))
    }

    #[tokio::test]
    async fn an_unset_setting_returns_its_default() {
        assert_eq!(settings().get(&REFRESH_MINUTES).await.unwrap(), 5);
    }

    #[tokio::test]
    async fn a_stored_value_wins_over_the_default() {
        let settings = settings();
        settings.set(&REFRESH_MINUTES, &15).await.unwrap();

        assert_eq!(settings.get(&REFRESH_MINUTES).await.unwrap(), 15);
        assert_eq!(
            settings.customised_keys().await.unwrap(),
            vec!["sync.refresh_minutes"]
        );
    }

    #[tokio::test]
    async fn resetting_restores_the_default() {
        let settings = settings();
        settings.set(&REFRESH_MINUTES, &15).await.unwrap();
        settings.reset(&REFRESH_MINUTES).await.unwrap();

        assert_eq!(settings.get(&REFRESH_MINUTES).await.unwrap(), 5);
        assert!(settings.customised_keys().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_undecodable_stored_value_falls_back_to_the_default() {
        let settings = settings();
        // Simulates a setting whose type changed between releases.
        settings
            .set(
                &Setting::<String>::new("sync.refresh_minutes", String::new),
                &"often".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(settings.get(&REFRESH_MINUTES).await.unwrap(), 5);
    }
}
