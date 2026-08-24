use crate::{Record, Storage, StorageKey};
use async_trait::async_trait;
use origin_core::Result;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Non-persistent storage for tests and headless runs.
#[derive(Debug, Default)]
pub struct MemoryStorage {
    records: RwLock<HashMap<StorageKey, Record>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn get(&self, key: &StorageKey) -> Result<Option<Record>> {
        Ok(self.records.read().await.get(key).cloned())
    }

    async fn put(&self, key: &StorageKey, record: Record) -> Result<()> {
        self.records.write().await.insert(key.clone(), record);
        Ok(())
    }

    async fn delete(&self, key: &StorageKey) -> Result<()> {
        self.records.write().await.remove(key);
        Ok(())
    }

    async fn keys(&self, namespace: &str) -> Result<Vec<StorageKey>> {
        Ok(self
            .records
            .read()
            .await
            .keys()
            .filter(|key| key.namespace() == namespace)
            .cloned()
            .collect())
    }

    async fn clear(&self, namespace: &str) -> Result<()> {
        self.records
            .write()
            .await
            .retain(|key, _| key.namespace() != namespace);
        Ok(())
    }

    async fn clear_prefix(&self, prefix: &str) -> Result<usize> {
        let mut records = self.records.write().await;
        let before = records.len();
        records.retain(|key, _| !key.namespace().starts_with(prefix));
        Ok(before - records.len())
    }
}
