use crate::{Record, Storage, StorageKey};
use origin_domain::{AppError, Clock, Result};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use time::Duration;

/// Typed, TTL-aware access to [`Storage`].
///
/// Expiry is decided here rather than in the backend, so SQLite, memory and any future
/// backend agree on what "stale" means. Time comes from a [`Clock`], which makes TTL
/// behaviour testable without sleeping.
#[derive(Debug, Clone)]
pub struct Cache {
    storage: Arc<dyn Storage>,
    clock: Arc<dyn Clock>,
}

impl Cache {
    pub fn new(storage: Arc<dyn Storage>, clock: Arc<dyn Clock>) -> Self {
        Self { storage, clock }
    }

    /// Fresh value, or `None` when absent or expired.
    pub async fn get<T: DeserializeOwned>(&self, key: &StorageKey) -> Result<Option<T>> {
        let Some(record) = self.storage.get(key).await? else {
            return Ok(None);
        };

        if record.is_expired_at(self.clock.now()) {
            return Ok(None);
        }

        let value = serde_json::from_str(&record.value)
            .map_err(|error| AppError::storage(format!("cannot decode {key:?}: {error}")))?;
        Ok(Some(value))
    }

    /// Store a value. `ttl` of `None` means it stays until explicitly invalidated.
    pub async fn put<T: Serialize>(
        &self,
        key: &StorageKey,
        value: &T,
        ttl: Option<Duration>,
    ) -> Result<()> {
        let encoded = serde_json::to_string(value)
            .map_err(|error| AppError::storage(format!("cannot encode {key:?}: {error}")))?;

        let now = self.clock.now();
        let mut record = Record::new(encoded, now);
        if let Some(ttl) = ttl {
            record = record.expiring_at(now + ttl);
        }

        self.storage.put(key, record).await
    }

    pub async fn invalidate(&self, key: &StorageKey) -> Result<()> {
        self.storage.delete(key).await
    }

    pub async fn invalidate_namespace(&self, namespace: &str) -> Result<()> {
        self.storage.clear(namespace).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryStorage;
    use origin_domain::testing::FakeClock;
    use time::macros::datetime;

    fn cache() -> (Cache, Arc<FakeClock>) {
        let clock = Arc::new(FakeClock::new(datetime!(2026-08-23 10:00 UTC)));
        let cache = Cache::new(Arc::new(MemoryStorage::new()), clock.clone());
        (cache, clock)
    }

    #[tokio::test]
    async fn a_value_survives_until_its_ttl_expires() {
        let (cache, clock) = cache();
        let key = StorageKey::new("github", "notifications");

        cache
            .put(&key, &vec!["a", "b"], Some(Duration::minutes(5)))
            .await
            .unwrap();

        clock.advance(Duration::minutes(4));
        let fresh: Option<Vec<String>> = cache.get(&key).await.unwrap();
        assert_eq!(
            fresh.as_deref(),
            Some(&["a".to_string(), "b".to_string()][..])
        );

        clock.advance(Duration::minutes(2));
        let stale: Option<Vec<String>> = cache.get(&key).await.unwrap();
        assert_eq!(
            stale, None,
            "the value must be treated as stale after its TTL"
        );
    }

    #[tokio::test]
    async fn a_value_without_ttl_never_expires() {
        let (cache, clock) = cache();
        let key = StorageKey::new("settings", "theme");

        cache.put(&key, &"dark", None).await.unwrap();
        clock.advance(Duration::days(365));

        assert_eq!(
            cache.get::<String>(&key).await.unwrap().as_deref(),
            Some("dark")
        );
    }

    #[tokio::test]
    async fn decoding_a_value_as_the_wrong_type_is_a_storage_error() {
        let (cache, _clock) = cache();
        let key = StorageKey::new("github", "count");
        cache.put(&key, &"not-a-number", None).await.unwrap();

        let error = cache.get::<u32>(&key).await.unwrap_err();
        assert_eq!(error.kind(), origin_domain::ErrorKind::Storage);
    }
}
