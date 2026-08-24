//! Credentials in the operating system's own credential store.
//!
//! | Platform | Backend |
//! | --- | --- |
//! | macOS | Keychain |
//! | Windows | Credential Manager |
//! | Linux | Secret Service (D-Bus) |
//!
//! The backend is chosen by the `keyring` crate at compile time. Callers see only
//! [`origin_secrets::SecretStore`] and never learn which one they got.

use async_trait::async_trait;
use keyring::Entry;
use origin_core::{AppError, Result};
use origin_secrets::{Secret, SecretKey, SecretStore};

/// System credential store, scoped to one application.
///
/// `service_prefix` keeps two Origin applications on the same machine from reading
/// each other's credentials.
#[derive(Debug, Clone)]
pub struct SystemSecretStore {
    service_prefix: String,
}

impl SystemSecretStore {
    pub fn new(service_prefix: impl Into<String>) -> Self {
        Self {
            service_prefix: service_prefix.into(),
        }
    }

    fn entry(&self, key: &SecretKey) -> Result<Entry> {
        let service = format!("{}.{}", self.service_prefix, key.namespace());
        Entry::new(&service, key.name()).map_err(to_app_error)
    }
}

#[async_trait]
impl SecretStore for SystemSecretStore {
    async fn get(&self, key: &SecretKey) -> Result<Option<Secret>> {
        let entry = self.entry(key)?;
        blocking(move || match entry.get_password() {
            Ok(password) => Ok(Some(Secret::new(password))),
            // A missing credential is a normal outcome, not a failure.
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(to_app_error(error)),
        })
        .await
    }

    async fn set(&self, key: &SecretKey, value: Secret) -> Result<()> {
        let entry = self.entry(key)?;
        blocking(move || entry.set_password(value.expose()).map_err(to_app_error)).await
    }

    async fn delete(&self, key: &SecretKey) -> Result<()> {
        let entry = self.entry(key)?;
        blocking(move || match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(to_app_error(error)),
        })
        .await
    }
}

/// Keychain access is blocking and can show a system prompt, so it must not run on
/// the async runtime's worker threads.
async fn blocking<T, F>(operation: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|error| AppError::internal(format!("credential task failed: {error}")))?
}

/// Translates keyring failures into the Origin error model.
///
/// A denied keychain prompt is an authentication problem the user can fix, not an
/// internal error — the UI needs that distinction to show the right message.
fn to_app_error(error: keyring::Error) -> AppError {
    match error {
        keyring::Error::NoEntry => AppError::Authentication("no credential stored".to_owned()),
        keyring::Error::NoDefaultStore => {
            AppError::Configuration("no credential store available on this system".to_owned())
        }
        keyring::Error::Ambiguous(_) => AppError::Storage(
            "several credentials matched — the credential store needs manual cleanup".to_owned(),
        ),
        keyring::Error::PlatformFailure(inner) => {
            AppError::Storage(format!("credential store unavailable: {inner}"))
        }
        keyring::Error::NoStorageAccess(inner) => {
            AppError::Permission(format!("credential store access denied: {inner}"))
        }
        other => AppError::Storage(other.to_string()),
    }
}
