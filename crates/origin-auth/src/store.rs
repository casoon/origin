use crate::TokenSet;
use crate::token::StoredTokenSet;
use origin_domain::{AccountId, AppError, ConnectorId, Result};
use origin_secrets::{Secret, SecretKey, SecretStore};
use std::sync::Arc;

/// Credentials in the OS credential store, one entry per account (ADR-0016).
///
/// Tokens never touch the application database (ADR-0008); revoking one account never
/// affects another.
#[derive(Debug, Clone)]
pub struct TokenStore {
    secrets: Arc<dyn SecretStore>,
}

impl TokenStore {
    pub fn new(secrets: Arc<dyn SecretStore>) -> Self {
        Self { secrets }
    }

    fn key(connector: &ConnectorId, account: &AccountId) -> SecretKey {
        SecretKey::new(format!("oauth.{connector}"), account.as_str())
    }

    pub async fn load(
        &self,
        connector: &ConnectorId,
        account: &AccountId,
    ) -> Result<Option<TokenSet>> {
        let Some(secret) = self.secrets.get(&Self::key(connector, account)).await? else {
            return Ok(None);
        };

        let stored: StoredTokenSet = serde_json::from_str(secret.expose()).map_err(|error| {
            // Corrupt credentials are not recoverable by retrying; the user has to
            // reconnect, and the error must say so.
            AppError::Authentication(format!(
                "stored credentials for {connector}/{account} are unreadable: {error}"
            ))
        })?;

        Ok(Some(stored.into()))
    }

    pub async fn save(
        &self,
        connector: &ConnectorId,
        account: &AccountId,
        tokens: &TokenSet,
    ) -> Result<()> {
        let encoded = serde_json::to_string(&StoredTokenSet::from(tokens))
            .map_err(|error| AppError::internal(format!("cannot encode credentials: {error}")))?;

        self.secrets
            .set(&Self::key(connector, account), Secret::new(encoded))
            .await
    }

    /// Remove an account's credentials. Used on disconnect and on unrecoverable
    /// authentication failures.
    pub async fn delete(&self, connector: &ConnectorId, account: &AccountId) -> Result<()> {
        self.secrets.delete(&Self::key(connector, account)).await
    }
}
