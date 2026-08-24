//! Test doubles for the authorization flow.

use crate::redirect::{AuthorizationCode, RedirectListener};
use async_trait::async_trait;
use origin_core::{AppError, Result};

/// A redirect listener that answers immediately with a canned outcome.
///
/// Lets the whole flow be tested without a socket, a browser or a provider.
#[derive(Debug)]
pub struct FakeRedirectListener {
    redirect_uri: String,
    outcome: Outcome,
}

#[derive(Debug)]
enum Outcome {
    Code(String),
    /// The state the provider will send back, to exercise the mismatch path.
    WrongState(String),
    Denied,
}

impl FakeRedirectListener {
    /// Returns `code` once the state matches.
    pub fn returning(code: impl Into<String>) -> Self {
        Self {
            redirect_uri: "http://127.0.0.1:1/callback".to_owned(),
            outcome: Outcome::Code(code.into()),
        }
    }

    /// Simulates a forged or stale redirect.
    pub fn with_state(state: impl Into<String>) -> Self {
        Self {
            redirect_uri: "http://127.0.0.1:1/callback".to_owned(),
            outcome: Outcome::WrongState(state.into()),
        }
    }

    /// Simulates the user pressing "Deny".
    pub fn denied() -> Self {
        Self {
            redirect_uri: "http://127.0.0.1:1/callback".to_owned(),
            outcome: Outcome::Denied,
        }
    }
}

#[async_trait]
impl RedirectListener for FakeRedirectListener {
    fn redirect_uri(&self) -> String {
        self.redirect_uri.clone()
    }

    async fn wait(&self, expected_state: &str) -> Result<AuthorizationCode> {
        match &self.outcome {
            Outcome::Code(code) => Ok(AuthorizationCode::new(code)),
            Outcome::WrongState(state) => Err(AppError::Authentication(format!(
                "unexpected state in redirect: got {state}, expected {expected_state}"
            ))),
            Outcome::Denied => Err(AppError::Authentication(
                "authorization was denied: access_denied".to_owned(),
            )),
        }
    }
}
