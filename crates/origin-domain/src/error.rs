//! The normalised error model.
//!
//! Adapters translate their native errors (`reqwest`, `rusqlite`, `tauri`, ...) into
//! [`AppError`] at the boundary. Nothing else ever reaches the frontend, so the UI can
//! offer one consistent error experience.

use serde::{Deserialize, Serialize};

pub type Result<T, E = AppError> = std::result::Result<T, E>;

/// Stable, serialisable classification of a failure.
///
/// The frontend switches on this — never on an error message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    Authentication,
    Permission,
    Network,
    Offline,
    RateLimited,
    Storage,
    ExternalService,
    Validation,
    Configuration,
    Internal,
}

impl ErrorKind {
    /// Whether retrying the same operation later can plausibly succeed.
    pub fn is_retryable(self) -> bool {
        matches!(
            self,
            Self::Network | Self::Offline | Self::RateLimited | Self::ExternalService
        )
    }

    /// Whether the user has to act (re-authenticate, grant access, fix config).
    pub fn needs_user_action(self) -> bool {
        matches!(
            self,
            Self::Authentication | Self::Permission | Self::Configuration | Self::Validation
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("authentication failed: {0}")]
    Authentication(String),

    #[error("permission denied: {0}")]
    Permission(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("offline: {0}")]
    Offline(String),

    #[error("rate limited: {message}")]
    RateLimited {
        message: String,
        /// Seconds to wait before the next attempt, if the service told us.
        retry_after_seconds: Option<u64>,
    },

    #[error("storage error: {0}")]
    Storage(String),

    #[error("external service error: {0}")]
    ExternalService(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("configuration error: {0}")]
    Configuration(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Authentication(_) => ErrorKind::Authentication,
            Self::Permission(_) => ErrorKind::Permission,
            Self::Network(_) => ErrorKind::Network,
            Self::Offline(_) => ErrorKind::Offline,
            Self::RateLimited { .. } => ErrorKind::RateLimited,
            Self::Storage(_) => ErrorKind::Storage,
            Self::ExternalService(_) => ErrorKind::ExternalService,
            Self::Validation(_) => ErrorKind::Validation,
            Self::Configuration(_) => ErrorKind::Configuration,
            Self::Internal(_) => ErrorKind::Internal,
        }
    }

    pub fn is_retryable(&self) -> bool {
        self.kind().is_retryable()
    }

    /// The IPC representation. This is what the frontend receives — never a raw
    /// `rusqlite::Error` or `reqwest::Error`.
    pub fn to_contract(&self) -> ErrorContract {
        ErrorContract {
            kind: self.kind(),
            message: self.to_string(),
            retryable: self.is_retryable(),
            needs_user_action: self.kind().needs_user_action(),
            retry_after_seconds: match self {
                Self::RateLimited {
                    retry_after_seconds,
                    ..
                } => *retry_after_seconds,
                _ => None,
            },
        }
    }

    pub fn storage(message: impl Into<String>) -> Self {
        Self::Storage(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration(message.into())
    }
}

/// Serialisable error payload crossing the IPC boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ErrorContract {
    pub kind: ErrorKind,
    pub message: String,
    pub retryable: bool,
    pub needs_user_action: bool,
    pub retry_after_seconds: Option<u64>,
}

impl From<&AppError> for ErrorContract {
    fn from(error: &AppError) -> Self {
        error.to_contract()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_is_retryable_and_carries_retry_after() {
        let error = AppError::RateLimited {
            message: "secondary rate limit".into(),
            retry_after_seconds: Some(60),
        };
        let contract = error.to_contract();

        assert_eq!(contract.kind, ErrorKind::RateLimited);
        assert!(contract.retryable);
        assert_eq!(contract.retry_after_seconds, Some(60));
    }

    #[test]
    fn authentication_needs_user_action_and_is_not_retryable() {
        let contract = AppError::Authentication("token expired".into()).to_contract();

        assert!(contract.needs_user_action);
        assert!(!contract.retryable);
    }
}
