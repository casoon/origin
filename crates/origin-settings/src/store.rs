use crate::SETTINGS_NAMESPACE;
use async_trait::async_trait;
use origin_domain::{Clock, Result};
use origin_storage::{Record, Storage, StorageKey};
use std::fmt::Debug;
use std::sync::Arc;

/// Raw persistence for settings. Values are JSON text.
///
/// Most code should use [`crate::Settings`] instead — this is the port an adapter
/// implements.
#[async_trait]
pub trait SettingsStore: Debug + Send + Sync + 'static {
    async fn get_raw(&self, key: &str) -> Result<Option<String>>;
    async fn set_raw(&self, key: &str, value: String) -> Result<()>;
    async fn remove(&self, key: &str) -> Result<()>;
    async fn keys(&self) -> Result<Vec<String>>;
}

/// Settings on top of any [`Storage`] backend.
///
/// Settings are user data, not cache: records are written without an expiry, so a
/// cache sweep can never drop them.
#[derive(Debug, Clone)]
pub struct StorageSettingsStore {
    storage: Arc<dyn Storage>,
    clock: Arc<dyn Clock>,
}

impl StorageSettingsStore {
    pub fn new(storage: Arc<dyn Storage>, clock: Arc<dyn Clock>) -> Self {
        Self { storage, clock }
    }

    fn key(name: &str) -> StorageKey {
        StorageKey::new(SETTINGS_NAMESPACE, name)
    }
}

#[async_trait]
impl SettingsStore for StorageSettingsStore {
    async fn get_raw(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .storage
            .get(&Self::key(key))
            .await?
            .map(|record| record.value))
    }

    async fn set_raw(&self, key: &str, value: String) -> Result<()> {
        self.storage
            .put(&Self::key(key), Record::new(value, self.clock.now()))
            .await
    }

    async fn remove(&self, key: &str) -> Result<()> {
        self.storage.delete(&Self::key(key)).await
    }

    async fn keys(&self) -> Result<Vec<String>> {
        Ok(self
            .storage
            .keys(SETTINGS_NAMESPACE)
            .await?
            .into_iter()
            .map(|key| key.key().to_owned())
            .collect())
    }
}
