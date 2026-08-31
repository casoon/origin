use crate::ACCOUNTS_NAMESPACE;
use origin_domain::{Account, AccountId, AppError, Clock, ConnectorId, Result};
use origin_storage::{Record, Storage, StorageKey};
use std::sync::Arc;

/// Persistence for the account list.
#[derive(Debug, Clone)]
pub struct AccountStore {
    storage: Arc<dyn Storage>,
    clock: Arc<dyn Clock>,
}

impl AccountStore {
    pub fn new(storage: Arc<dyn Storage>, clock: Arc<dyn Clock>) -> Self {
        Self { storage, clock }
    }

    fn key(account: &AccountId) -> StorageKey {
        StorageKey::new(ACCOUNTS_NAMESPACE, account.as_str())
    }

    pub async fn get(&self, account: &AccountId) -> Result<Option<Account>> {
        let Some(record) = self.storage.get(&Self::key(account)).await? else {
            return Ok(None);
        };

        serde_json::from_str(&record.value)
            .map(Some)
            .map_err(|error| AppError::storage(format!("account {account} is unreadable: {error}")))
    }

    pub async fn save(&self, account: &Account) -> Result<()> {
        let encoded = serde_json::to_string(account)
            .map_err(|error| AppError::storage(format!("cannot encode account: {error}")))?;

        // Accounts are user data, not cache: stored without an expiry so no cache sweep
        // can remove them.
        self.storage
            .put(
                &Self::key(&account.id),
                Record::new(encoded, self.clock.now()),
            )
            .await
    }

    pub async fn remove(&self, account: &AccountId) -> Result<()> {
        self.storage.delete(&Self::key(account)).await
    }

    /// Every account, across all connectors.
    pub async fn list(&self) -> Result<Vec<Account>> {
        let mut accounts = Vec::new();

        for key in self.storage.keys(ACCOUNTS_NAMESPACE).await? {
            let id = AccountId::new(key.key());
            match self.get(&id).await {
                Ok(Some(account)) => accounts.push(account),
                Ok(None) => {}
                // One unreadable record must not hide every other account; the user
                // would see an empty list and reconnect everything.
                Err(error) => tracing::warn!(%id, %error, "skipping unreadable account record"),
            }
        }

        accounts.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        Ok(accounts)
    }

    /// Accounts belonging to one connector.
    pub async fn list_for(&self, connector: &ConnectorId) -> Result<Vec<Account>> {
        Ok(self
            .list()
            .await?
            .into_iter()
            .filter(|account| &account.connector == connector)
            .collect())
    }
}
