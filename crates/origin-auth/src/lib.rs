//! OAuth 2.0 for native applications (ADR-0015).
//!
//! Implements the authorization code flow with PKCE, a loopback redirect, token
//! storage in the OS credential store, and transparent refresh.
//!
//! ```text
//! LoopbackRedirect::bind()      → redirect_uri
//! AuthorizationFlow::begin()    → authorization url + state + verifier
//! Opener::open_url()            → the user consents in their browser
//! RedirectListener::wait()      → code (state verified)
//! AuthorizationFlow::exchange() → TokenSet
//! TokenStore::save()            → OS credential store
//! ```
//!
//! Afterwards nothing calls the flow again: [`AccessTokenProvider`] hands out a valid
//! access token and refreshes it when it is about to expire.

mod config;
mod flow;
mod pkce;
mod provider;
mod redirect;
mod store;
mod token;

#[cfg(feature = "testing")]
pub mod testing;

pub use config::OAuthConfig;
pub use flow::{AuthorizationFlow, PendingAuthorization};
pub use pkce::Pkce;
pub use provider::AccessTokenProvider;
pub use redirect::{AuthorizationCode, RedirectListener};
pub use store::TokenStore;
pub use token::TokenSet;

/// Random bytes, base64url-encoded without padding.
///
/// Used for the PKCE verifier and the `state` parameter — both must be
/// unguessable, and both travel in URLs.
pub(crate) fn random_token(bytes: usize) -> origin_domain::Result<String> {
    use base64::Engine as _;

    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer).map_err(|error| {
        origin_domain::AppError::internal(format!("no secure randomness available: {error}"))
    })?;

    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buffer))
}
