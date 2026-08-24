use origin_secrets::Secret;

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
    pub fn new(
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

    pub fn with_authorization_param(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.extra_authorization_params
            .push((key.into(), value.into()));
        self
    }

    pub(crate) fn scope_parameter(&self) -> String {
        self.scopes.join(" ")
    }
}
