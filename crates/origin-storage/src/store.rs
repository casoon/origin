use async_trait::async_trait;
use origin_core::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use time::OffsetDateTime;

/// Addresses one record. The namespace groups records that are invalidated together,
/// e.g. `github.notifications`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StorageKey {
    namespace: String,
    key: String,
}

impl StorageKey {
    pub fn new(namespace: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            key: key.into(),
        }
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn key(&self) -> &str {
        &self.key
    }
}

/// A stored value plus its cache metadata. `value` is JSON text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub value: String,
    #[serde(with = "time::serde::rfc3339")]
    pub stored_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub expires_at: Option<OffsetDateTime>,
}

impl Record {
    pub fn new(value: impl Into<String>, stored_at: OffsetDateTime) -> Self {
        Self {
            value: value.into(),
            stored_at,
            expires_at: None,
        }
    }

    pub fn expiring_at(mut self, expires_at: OffsetDateTime) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Whether this record is stale at `now`. Records without an expiry never are.
    pub fn is_expired_at(&self, now: OffsetDateTime) -> bool {
        self.expires_at.is_some_and(|expires_at| now >= expires_at)
    }
}

/// Persistence for cache entries, read models and local state.
#[async_trait]
pub trait Storage: Debug + Send + Sync + 'static {
    /// Returns the record as stored, **including expired ones**. Callers that care
    /// about freshness go through [`crate::Cache`].
    async fn get(&self, key: &StorageKey) -> Result<Option<Record>>;

    async fn put(&self, key: &StorageKey, record: Record) -> Result<()>;

    /// Deleting a missing key succeeds.
    async fn delete(&self, key: &StorageKey) -> Result<()>;

    /// All keys in a namespace, in unspecified order.
    async fn keys(&self, namespace: &str) -> Result<Vec<StorageKey>>;

    /// Drop every record in a namespace.
    async fn clear(&self, namespace: &str) -> Result<()>;

    /// Drop every record whose namespace starts with `prefix`, and report how many.
    ///
    /// This is how disconnecting an account removes its data (ADR-0019): the caller
    /// passes [`crate::namespace::account_prefix`] and needs to know nothing about
    /// which namespaces each module wrote.
    async fn clear_prefix(&self, prefix: &str) -> Result<usize>;
}
