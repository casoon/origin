use crate::redirect::{AuthorizationCode, RedirectListener};
use crate::token::TokenResponse;
use crate::{OAuthConfig, Pkce, TokenSet, random_token};
use origin_domain::{AppError, Clock, Result};
use origin_http::{HttpClient, HttpRequest};
use origin_platform::Opener;
use std::sync::Arc;

/// A flow that has been started but not yet completed.
///
/// Holds the two values that must survive until the code comes back: the `state` used
/// to recognise our own redirect, and the PKCE verifier that proves we started it.
#[derive(Debug)]
pub struct PendingAuthorization {
    pub authorization_url: String,
    pub(crate) state: String,
    pub(crate) pkce: Pkce,
    pub(crate) redirect_uri: String,
}

/// The OAuth 2.0 authorization code flow with PKCE.
#[derive(Debug, Clone)]
pub struct AuthorizationFlow {
    config: OAuthConfig,
    http: Arc<dyn HttpClient>,
    clock: Arc<dyn Clock>,
}

impl AuthorizationFlow {
    pub fn new(config: OAuthConfig, http: Arc<dyn HttpClient>, clock: Arc<dyn Clock>) -> Self {
        Self {
            config,
            http,
            clock,
        }
    }

    /// Build the URL to send the user to.
    ///
    /// `redirect_uri` comes from an already-listening [`RedirectListener`].
    pub fn begin(&self, redirect_uri: impl Into<String>) -> Result<PendingAuthorization> {
        let redirect_uri = redirect_uri.into();
        let state = random_token(32)?;
        let pkce = Pkce::generate()?;

        let scope = self.config.scope_parameter();
        let mut parameters: Vec<(&str, &str)> = vec![
            ("response_type", "code"),
            ("client_id", &self.config.client_id),
            ("redirect_uri", &redirect_uri),
            ("state", &state),
            ("code_challenge", pkce.challenge()),
            ("code_challenge_method", "S256"),
        ];
        if !scope.is_empty() {
            parameters.push(("scope", &scope));
        }
        for (key, value) in &self.config.extra_authorization_params {
            parameters.push((key.as_str(), value.as_str()));
        }

        let authorization_url = HttpRequest::get(&self.config.authorization_endpoint)
            .query(&parameters)
            .url;

        Ok(PendingAuthorization {
            authorization_url,
            state,
            pkce,
            redirect_uri,
        })
    }

    /// Trade the authorization code for tokens.
    pub async fn exchange(
        &self,
        pending: &PendingAuthorization,
        code: &AuthorizationCode,
    ) -> Result<TokenSet> {
        let mut fields: Vec<(&str, &str)> = vec![
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", &pending.redirect_uri),
            ("client_id", &self.config.client_id),
            ("code_verifier", pending.pkce.verifier()),
        ];
        if let Some(secret) = &self.config.client_secret {
            fields.push(("client_secret", secret.expose()));
        }

        self.post_token_request(&fields).await
    }

    /// Exchange a refresh token for a new access token.
    pub async fn refresh(&self, refresh_token: &str) -> Result<TokenSet> {
        let mut fields: Vec<(&str, &str)> = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.config.client_id),
        ];
        if let Some(secret) = &self.config.client_secret {
            fields.push(("client_secret", secret.expose()));
        }

        self.post_token_request(&fields).await
    }

    /// Run the whole flow: open the browser, wait for the redirect, exchange the code.
    pub async fn authorize(
        &self,
        listener: &dyn RedirectListener,
        opener: &dyn Opener,
    ) -> Result<TokenSet> {
        let pending = self.begin(listener.redirect_uri())?;

        tracing::info!("opening browser for authorization");
        opener.open_url(&pending.authorization_url).await?;

        let code = listener.wait(&pending.state).await?;
        let tokens = self.exchange(&pending, &code).await?;

        tracing::info!(scopes = ?tokens.scopes, "authorization complete");
        Ok(tokens)
    }

    async fn post_token_request(&self, fields: &[(&str, &str)]) -> Result<TokenSet> {
        let request = HttpRequest::post(&self.config.token_endpoint)
            .header("accept", "application/json")
            .form(fields);

        let now = self.clock.now();
        let response = self.http.send(request).await?;

        if !response.is_success() {
            return Err(token_endpoint_error(&response, now));
        }

        Ok(response.json::<TokenResponse>()?.into_token_set(now))
    }
}

/// Only `invalid_grant` proves that the stored authorization is no longer usable.
/// Provider outages and malformed error responses must not log the user out.
fn token_endpoint_error(
    response: &origin_http::HttpResponse,
    now: time::OffsetDateTime,
) -> AppError {
    #[derive(serde::Deserialize)]
    struct OAuthError {
        error: Option<String>,
        error_description: Option<String>,
    }

    if let Ok(error) = response.json::<OAuthError>() {
        let code = error.error.clone();
        let message = error
            .error_description
            .or(error.error)
            .unwrap_or_else(|| format!("token endpoint returned http {}", response.status));

        return match code.as_deref() {
            Some("invalid_grant") => AppError::Authentication(message),
            Some("invalid_client" | "unauthorized_client" | "unsupported_grant_type") => {
                AppError::Configuration(message)
            }
            Some("access_denied" | "invalid_scope") => AppError::Permission(message),
            _ => AppError::ExternalService(message),
        };
    }

    match response.clone().error_for_status(now).unwrap_err() {
        AppError::Authentication(_) | AppError::Validation(_) => {
            AppError::ExternalService(format!("token endpoint returned http {}", response.status))
        }
        error => error,
    }
}
