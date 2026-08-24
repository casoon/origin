use async_trait::async_trait;
use origin_core::Result;
use std::fmt::Debug;

/// The authorization code handed back by the provider, after `state` was verified.
#[derive(Debug, Clone)]
pub struct AuthorizationCode(String);

impl AuthorizationCode {
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Receives the provider's redirect.
///
/// The implementation must be ready to receive *before* the authorization URL is
/// opened, which is why [`RedirectListener::redirect_uri`] is available immediately —
/// a loopback listener has already bound its port by then.
#[async_trait]
pub trait RedirectListener: Debug + Send + Sync {
    /// The `redirect_uri` to send to the authorization endpoint.
    fn redirect_uri(&self) -> String;

    /// Wait for the redirect and return the code.
    ///
    /// Implementations must reject a response whose `state` does not match
    /// `expected_state` — that check is what makes the flow immune to a forged
    /// redirect — and must surface an `error` parameter (the user pressed "Deny")
    /// rather than hanging.
    async fn wait(&self, expected_state: &str) -> Result<AuthorizationCode>;
}
