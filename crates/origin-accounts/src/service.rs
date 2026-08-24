use crate::store::AccountStore;
use origin_auth::{TokenSet, TokenStore};
use origin_core::{Account, AccountId, AccountStatus, AppError, Clock, ConnectorId, Result};
use origin_events::{AccountExpired, EventBus, PlatformEvent};
use origin_storage::{Storage, namespace};
use std::sync::Arc;

/// Connecting, listing and disconnecting accounts.
///
/// Keeps the two halves of an account consistent: the record in storage and the
/// credentials in the credential store.
#[derive(Debug, Clone)]
pub struct AccountService {
    accounts: AccountStore,
    tokens: TokenStore,
    events: EventBus,
    storage: Arc<dyn Storage>,
    clock: Arc<dyn Clock>,
}

impl AccountService {
    pub fn new(
        accounts: AccountStore,
        tokens: TokenStore,
        events: EventBus,
        storage: Arc<dyn Storage>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            accounts,
            tokens,
            events,
            storage,
            clock,
        }
    }

    /// Register a freshly authorized account.
    ///
    /// Credentials are written first: an account record without credentials is a broken
    /// entry the user has to notice and remove, while credentials without a record are
    /// merely orphaned and get overwritten on the next connect.
    pub async fn connect(
        &self,
        connector: &ConnectorId,
        display_name: impl Into<String>,
        tokens: &TokenSet,
    ) -> Result<Account> {
        let account = Account {
            id: AccountId::generate(),
            connector: connector.clone(),
            display_name: display_name.into(),
            status: AccountStatus::Active,
            connected_at: self.clock.now(),
        };

        self.tokens.save(connector, &account.id, tokens).await?;
        self.accounts.save(&account).await?;

        tracing::info!(
            %connector,
            account = %account.id,
            "account connected"
        );
        Ok(account)
    }

    pub async fn list(&self) -> Result<Vec<Account>> {
        self.accounts.list().await
    }

    pub async fn list_for(&self, connector: &ConnectorId) -> Result<Vec<Account>> {
        self.accounts.list_for(connector).await
    }

    pub async fn get(&self, account: &AccountId) -> Result<Account> {
        self.accounts
            .get(account)
            .await?
            .ok_or_else(|| AppError::validation(format!("unknown account {account}")))
    }

    /// Remove an account, its credentials and everything stored under it.
    ///
    /// The namespace convention (ADR-0019) is what makes the last part mechanical: all
    /// account data lives under `acct.<connector>.<account>.`, so no module has to
    /// register which namespaces it wrote.
    pub async fn disconnect(&self, account: &AccountId) -> Result<()> {
        let record = self.get(account).await?;

        // Credentials go first: if a later step fails, the worst case is a
        // disconnected account still listed, not a live token nobody can see.
        self.tokens.delete(&record.connector, account).await?;

        let prefix = namespace::account_prefix(&record.connector, account);
        let removed = self.storage.clear_prefix(&prefix).await?;

        self.accounts.remove(account).await?;

        tracing::info!(
            connector = %record.connector,
            %account,
            records_removed = removed,
            "account disconnected"
        );
        Ok(())
    }

    /// Mark an account as needing re-authentication.
    ///
    /// Called when a connector reports that credentials are no longer valid. The
    /// credentials are kept: a provider outage can look like an expired token, and
    /// deleting them would force an unnecessary reconnect.
    pub async fn mark_expired(&self, account: &AccountId) -> Result<()> {
        let mut record = self.get(account).await?;

        if record.status == AccountStatus::Expired {
            return Ok(());
        }

        record.status = AccountStatus::Expired;
        self.accounts.save(&record).await?;

        let _ = self
            .events
            .publish(PlatformEvent::AccountExpired(AccountExpired {
                account: account.clone(),
                connector: record.connector.clone(),
            }));

        tracing::warn!(connector = %record.connector, %account, "account marked as expired");
        Ok(())
    }

    /// Mark an account usable again after a successful verification.
    pub async fn mark_active(&self, account: &AccountId) -> Result<()> {
        let mut record = self.get(account).await?;

        if record.status == AccountStatus::Active {
            return Ok(());
        }

        record.status = AccountStatus::Active;
        self.accounts.save(&record).await
    }
}
