use crate::SyncTarget;
use origin_domain::{AppError, Clock, Result, SyncState};
use origin_storage::{Record, Storage, StorageKey};
use std::sync::Arc;

/// Persistence for [`SyncState`].
///
/// Written without an expiry: sync bookkeeping is not cache, and losing it would mean
/// re-fetching everything after a cache sweep.
#[derive(Debug, Clone)]
pub(crate) struct SyncStateStore {
    storage: Arc<dyn Storage>,
    clock: Arc<dyn Clock>,
}

impl SyncStateStore {
    pub(crate) fn new(storage: Arc<dyn Storage>, clock: Arc<dyn Clock>) -> Self {
        Self { storage, clock }
    }

    fn key(target: &SyncTarget) -> StorageKey {
        StorageKey::new(target.namespace(), &target.name)
    }

    pub(crate) async fn load(&self, target: &SyncTarget) -> Result<SyncState> {
        let Some(record) = self.storage.get(&Self::key(target)).await? else {
            return Ok(SyncState::default());
        };

        match serde_json::from_str(&record.value) {
            Ok(state) => Ok(state),
            // Unreadable bookkeeping must not block syncing: start over rather than
            // leaving the target permanently broken.
            Err(error) => {
                tracing::warn!(%target, %error, "sync state unreadable, starting fresh");
                Ok(SyncState::default())
            }
        }
    }

    pub(crate) async fn save(&self, target: &SyncTarget, state: &SyncState) -> Result<()> {
        let encoded = serde_json::to_string(state)
            .map_err(|error| AppError::storage(format!("cannot encode sync state: {error}")))?;

        self.storage
            .put(&Self::key(target), Record::new(encoded, self.clock.now()))
            .await
    }
}
