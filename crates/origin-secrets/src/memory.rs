use crate::{Secret, SecretKey, SecretStore};
use async_trait::async_trait;
use origin_domain::Result;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// In-memory secret store for tests and for the headless/CI path.
///
/// Never use it in a shipped application: the values live in plain process memory
/// and are gone on restart.
#[derive(Debug, Default)]
pub struct MemorySecretStore {
    entries: RwLock<HashMap<SecretKey, Secret>>,
}

impl MemorySecretStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SecretStore for MemorySecretStore {
    async fn get(&self, key: &SecretKey) -> Result<Option<Secret>> {
        Ok(self.entries.read().await.get(key).cloned())
    }

    async fn set(&self, key: &SecretKey, value: Secret) -> Result<()> {
        self.entries.write().await.insert(key.clone(), value);
        Ok(())
    }

    async fn delete(&self, key: &SecretKey) -> Result<()> {
        self.entries.write().await.remove(key);
        Ok(())
    }
}
