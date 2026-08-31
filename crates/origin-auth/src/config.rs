use origin_domain::{AppError, Result};
use origin_secrets::Secret;

/// Authorization parameters the flow itself sets (`crates/origin-auth/src/flow.rs`).
/// Letting a provider-specific extra param reuse one of these would let it silently
/// override `state` or `code_challenge` — exactly the values PKCE and CSRF protection
/// depend on being what the flow generated, not what a caller passed in.
const RESERVED_AUTHORIZATION_PARAMS: &[&str] = &[
    "response_type",
    "client_id",
    "redirect_uri",
    "state",
    "code_challenge",
    "code_challenge_method",
    "scope",
];

/// Everything needed to talk to one OAuth provider.
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub client_id: String,

    /// Present only for confidential clients.
    ///
    /// A desktop application cannot keep a secret from its user, so most providers
    /// issue public clients and PKCE replaces the secret entirely. When a provider
    /// insists on one, it is stored like any other credential.
    pub client_secret: Option<Secret>,

    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub scopes: Vec<String>,

    /// Provider-specific authorization parameters, e.g. Google's
    /// `access_type=offline` (without which no refresh token is issued).
    pub extra_authorization_params: Vec<(String, String)>,
}

impl OAuthConfig {
    /// Fails if either endpoint is not `https://` — a plaintext authorization or token
    /// endpoint lets a network observer read codes, tokens and (for a confidential
    /// client) the client secret. Use [`OAuthConfig::insecure_for_testing`] for a local
    /// provider double.
    pub fn new(
        client_id: impl Into<String>,
        authorization_endpoint: impl Into<String>,
        token_endpoint: impl Into<String>,
    ) -> Result<Self> {
        let authorization_endpoint = authorization_endpoint.into();
        let token_endpoint = token_endpoint.into();
        require_https("authorization_endpoint", &authorization_endpoint)?;
        require_https("token_endpoint", &token_endpoint)?;

        Ok(Self::new_unchecked(
            client_id,
            authorization_endpoint,
            token_endpoint,
        ))
    }

    /// Skips the HTTPS requirement [`OAuthConfig::new`] enforces. For a local provider
    /// double in a test, never for a real endpoint — the override is a deliberate,
    /// separate call so it cannot happen by omission.
    pub fn insecure_for_testing(
        client_id: impl Into<String>,
        authorization_endpoint: impl Into<String>,
        token_endpoint: impl Into<String>,
    ) -> Self {
        Self::new_unchecked(client_id, authorization_endpoint, token_endpoint)
    }

    fn new_unchecked(
        client_id: impl Into<String>,
        authorization_endpoint: impl Into<String>,
        token_endpoint: impl Into<String>,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: None,
            authorization_endpoint: authorization_endpoint.into(),
            token_endpoint: token_endpoint.into(),
            scopes: Vec::new(),
            extra_authorization_params: Vec::new(),
        }
    }

    pub fn with_scopes<S: Into<String>>(mut self, scopes: impl IntoIterator<Item = S>) -> Self {
        self.scopes = scopes.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_client_secret(mut self, secret: Secret) -> Self {
        self.client_secret = Some(secret);
        self
    }

    /// Adds a provider-specific authorization parameter, e.g. Google's
    /// `access_type=offline`.
    ///
    /// Fails if `key` is one the flow already sets itself (`response_type`, `client_id`,
    /// `redirect_uri`, `state`, `code_challenge`, `code_challenge_method`, `scope`);
    /// silently letting a later value win would mean a provider-specific param can
    /// override `state` or `code_challenge` without that ever being a visible decision.
    pub fn with_authorization_param(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self> {
        let key = key.into();
        if RESERVED_AUTHORIZATION_PARAMS.contains(&key.as_str()) {
            return Err(AppError::configuration(format!(
                "`{key}` is set by the authorization flow itself and cannot be overridden"
            )));
        }

        self.extra_authorization_params.push((key, value.into()));
        Ok(self)
    }

    pub(crate) fn scope_parameter(&self) -> String {
        self.scopes.join(" ")
    }
}

fn require_https(field: &str, endpoint: &str) -> Result<()> {
    if endpoint.starts_with("https://") {
        Ok(())
    } else {
        Err(AppError::configuration(format!(
            "{field} must be https, got `{endpoint}`"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use origin_domain::ErrorKind;

    #[test]
    fn a_plaintext_authorization_endpoint_is_rejected() {
        let error = OAuthConfig::new(
            "client",
            "http://provider.example/authorize",
            "https://provider.example/token",
        )
        .unwrap_err();

        assert_eq!(error.kind(), ErrorKind::Configuration);
        assert!(error.to_string().contains("authorization_endpoint"));
    }

    #[test]
    fn a_plaintext_token_endpoint_is_rejected() {
        let error = OAuthConfig::new(
            "client",
            "https://provider.example/authorize",
            "http://provider.example/token",
        )
        .unwrap_err();

        assert_eq!(error.kind(), ErrorKind::Configuration);
        assert!(error.to_string().contains("token_endpoint"));
    }

    #[test]
    fn https_endpoints_are_accepted() {
        assert!(
            OAuthConfig::new(
                "client",
                "https://provider.example/authorize",
                "https://provider.example/token"
            )
            .is_ok()
        );
    }

    #[test]
    fn insecure_for_testing_skips_the_https_requirement() {
        let config = OAuthConfig::insecure_for_testing(
            "client",
            "http://127.0.0.1:4000/authorize",
            "http://127.0.0.1:4000/token",
        );

        assert_eq!(
            config.authorization_endpoint,
            "http://127.0.0.1:4000/authorize"
        );
    }

    #[test]
    fn a_reserved_authorization_param_is_rejected() {
        let error = OAuthConfig::insecure_for_testing(
            "client",
            "http://provider.example/authorize",
            "http://provider.example/token",
        )
        .with_authorization_param("state", "attacker-controlled")
        .unwrap_err();

        assert_eq!(error.kind(), ErrorKind::Configuration);
        assert!(error.to_string().contains("state"));
    }

    #[test]
    fn a_provider_specific_authorization_param_is_accepted() {
        let config = OAuthConfig::insecure_for_testing(
            "client",
            "http://provider.example/authorize",
            "http://provider.example/token",
        )
        .with_authorization_param("access_type", "offline")
        .unwrap();

        assert_eq!(
            config.extra_authorization_params,
            vec![("access_type".to_owned(), "offline".to_owned())]
        );
    }
}
