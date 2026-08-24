use crate::{AuthorizationFlow, TokenSet, TokenStore};
use origin_core::{AccountId, AppError, Clock, ConnectorId, Result};
use origin_secrets::Secret;
use std::sync::Arc;
use time::Duration;
use tokio::sync::Mutex;

/// Refresh this long before the token actually expires, to survive clock skew and the
/// time the request spends in flight.
const REFRESH_SKEW: Duration = Duration::seconds(60);

/// Hands out a valid access token, refreshing when needed.
///
/// Connectors depend on this rather than on [`TokenStore`], so nothing outside this
/// type has to reason about expiry.
#[derive(Debug)]
pub struct AccessTokenProvider {
    connector: ConnectorId,
    flow: AuthorizationFlow,
    tokens: TokenStore,
    clock: Arc<dyn Clock>,
    /// Serialises refreshes.
    ///
    /// Without it, ten concurrent requests on an expired token trigger ten refreshes,
    /// and a provider that rotates refresh tokens invalidates nine of them.
    refresh_lock: Mutex<()>,
}

impl AccessTokenProvider {
    pub fn new(
        connector: ConnectorId,
        flow: AuthorizationFlow,
        tokens: TokenStore,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            connector,
            flow,
            tokens,
            clock,
            refresh_lock: Mutex::new(()),
        }
    }

    /// A usable access token for `account`.
    ///
    /// Fails with `Authentication` when the account has never been connected, or when
    /// the token expired and cannot be refreshed — both mean the user has to act.
    pub async fn access_token(&self, account: &AccountId) -> Result<Secret> {
        let current = self.load(account).await?;

        if !current.expires_within(self.clock.as_ref(), REFRESH_SKEW) {
            return Ok(current.access_token);
        }

        let _guard = self.refresh_lock.lock().await;

        // Another task may have refreshed while we waited for the lock.
        let current = self.load(account).await?;
        if !current.expires_within(self.clock.as_ref(), REFRESH_SKEW) {
            return Ok(current.access_token);
        }

        let refreshed = self.refresh(account, &current).await?;
        Ok(refreshed.access_token)
    }

    /// Force a refresh, e.g. after a `401` from an API that expires tokens early.
    pub async fn force_refresh(&self, account: &AccountId) -> Result<Secret> {
        let _guard = self.refresh_lock.lock().await;
        let current = self.load(account).await?;
        Ok(self.refresh(account, &current).await?.access_token)
    }

    /// Store the tokens of a freshly authorized account.
    pub async fn store(&self, account: &AccountId, tokens: &TokenSet) -> Result<()> {
        self.tokens.save(&self.connector, account, tokens).await
    }

    /// Forget an account's credentials.
    pub async fn forget(&self, account: &AccountId) -> Result<()> {
        self.tokens.delete(&self.connector, account).await
    }

    async fn load(&self, account: &AccountId) -> Result<TokenSet> {
        self.tokens
            .load(&self.connector, account)
            .await?
            .ok_or_else(|| {
                AppError::Authentication(format!(
                    "account {account} is not connected to {}",
                    self.connector
                ))
            })
    }

    async fn refresh(&self, account: &AccountId, current: &TokenSet) -> Result<TokenSet> {
        let Some(refresh_token) = current.refresh_token.as_ref() else {
            return Err(AppError::Authentication(format!(
                "the session for {account} expired and cannot be renewed — please \
                 reconnect the account"
            )));
        };

        tracing::info!(connector = %self.connector, %account, "refreshing access token");

        let refreshed = match self.flow.refresh(refresh_token.expose()).await {
            Ok(refreshed) => refreshed,
            Err(error) => {
                // A rejected refresh token is final: keeping it would retry forever.
                if error.kind() == origin_core::ErrorKind::Authentication {
                    tracing::warn!(%account, "refresh token rejected, discarding credentials");
                    self.tokens.delete(&self.connector, account).await?;
                }
                return Err(error);
            }
        };

        let merged = current.merge_refreshed(refreshed);
        self.tokens.save(&self.connector, account, &merged).await?;
        Ok(merged)
    }
}
