use crate::{Secret, SecretKey};
use async_trait::async_trait;
use origin_domain::Result;
use std::fmt::Debug;

/// Persistent credential storage.
///
/// Implementations must be safe to call from multiple tasks at once, and must pass
/// the shared contract suite in [`crate::contract`].
#[async_trait]
pub trait SecretStore: Debug + Send + Sync + 'static {
    /// `Ok(None)` when no secret is stored under `key` — a missing key is not an error.
    async fn get(&self, key: &SecretKey) -> Result<Option<Secret>>;

    /// Store `value`, replacing any existing secret under `key`.
    async fn set(&self, key: &SecretKey, value: Secret) -> Result<()>;

    /// Remove the secret. Deleting a missing key succeeds.
    async fn delete(&self, key: &SecretKey) -> Result<()>;
}
