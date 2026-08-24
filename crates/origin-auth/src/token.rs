use origin_core::Clock;
use origin_secrets::Secret;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

/// The credentials of one authenticated account.
#[derive(Debug, Clone)]
pub struct TokenSet {
    pub access_token: Secret,
    /// Absent when the provider issues no refresh token — the user then has to
    /// re-authenticate once the access token expires.
    pub refresh_token: Option<Secret>,
    pub token_type: String,
    /// Absent when the provider does not say; such tokens are treated as long-lived.
    pub expires_at: Option<OffsetDateTime>,
    /// Scopes the provider actually granted, which can be fewer than requested.
    pub scopes: Vec<String>,
}

impl TokenSet {
    /// Whether the access token is expired, or will be within `skew`.
    ///
    /// The skew matters: a token that is valid for another two seconds when checked
    /// will be rejected by the time the request arrives.
    pub fn expires_within(&self, clock: &dyn Clock, skew: Duration) -> bool {
        match self.expires_at {
            None => false,
            Some(expires_at) => clock.now() + skew >= expires_at,
        }
    }

    pub fn can_refresh(&self) -> bool {
        self.refresh_token.is_some()
    }

    /// Apply a refresh response.
    ///
    /// Many providers omit `refresh_token` when refreshing, meaning "keep using the one
    /// you have". Dropping it there would log the user out on the next refresh.
    pub(crate) fn merge_refreshed(&self, refreshed: TokenSet) -> TokenSet {
        TokenSet {
            refresh_token: refreshed
                .refresh_token
                .or_else(|| self.refresh_token.clone()),
            scopes: if refreshed.scopes.is_empty() {
                self.scopes.clone()
            } else {
                refreshed.scopes
            },
            ..refreshed
        }
    }
}

/// What a token endpoint returns (RFC 6749 §5.1).
#[derive(Debug, Deserialize)]
pub(crate) struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: Option<String>,
    /// Lifetime in seconds from now.
    pub expires_in: Option<i64>,
    /// Space-separated list.
    pub scope: Option<String>,
}

impl TokenResponse {
    pub(crate) fn into_token_set(self, now: OffsetDateTime) -> TokenSet {
        TokenSet {
            access_token: Secret::new(self.access_token),
            refresh_token: self.refresh_token.map(Secret::new),
            token_type: self.token_type.unwrap_or_else(|| "Bearer".to_owned()),
            expires_at: self
                .expires_in
                .map(|seconds| now + Duration::seconds(seconds)),
            scopes: self
                .scope
                .map(|scope| scope.split_whitespace().map(str::to_owned).collect())
                .unwrap_or_default(),
        }
    }
}

/// On-disk shape. Kept separate from [`TokenSet`] so `Secret` never gains a
/// `Serialize` implementation — that would make leaking one an accident away.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct StoredTokenSet {
    access_token: String,
    refresh_token: Option<String>,
    token_type: String,
    #[serde(with = "time::serde::rfc3339::option")]
    expires_at: Option<OffsetDateTime>,
    scopes: Vec<String>,
}

impl From<&TokenSet> for StoredTokenSet {
    fn from(tokens: &TokenSet) -> Self {
        Self {
            access_token: tokens.access_token.expose().to_owned(),
            refresh_token: tokens
                .refresh_token
                .as_ref()
                .map(|token| token.expose().to_owned()),
            token_type: tokens.token_type.clone(),
            expires_at: tokens.expires_at,
            scopes: tokens.scopes.clone(),
        }
    }
}

impl From<StoredTokenSet> for TokenSet {
    fn from(stored: StoredTokenSet) -> Self {
        Self {
            access_token: Secret::new(stored.access_token),
            refresh_token: stored.refresh_token.map(Secret::new),
            token_type: stored.token_type,
            expires_at: stored.expires_at,
            scopes: stored.scopes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use origin_core::testing::FakeClock;
    use time::macros::datetime;

    const NOW: OffsetDateTime = datetime!(2026-08-23 10:00 UTC);

    fn token_set(expires_in: Option<i64>, refresh: Option<&str>) -> TokenSet {
        TokenResponse {
            access_token: "access".to_owned(),
            refresh_token: refresh.map(str::to_owned),
            token_type: None,
            expires_in,
            scope: Some("repo read:org".to_owned()),
        }
        .into_token_set(NOW)
    }

    #[test]
    fn expiry_accounts_for_clock_skew() {
        let clock = FakeClock::new(NOW);
        let tokens = token_set(Some(120), None);

        assert!(!tokens.expires_within(&clock, Duration::seconds(60)));

        clock.advance(Duration::seconds(61));
        assert!(
            tokens.expires_within(&clock, Duration::seconds(60)),
            "a token expiring within the skew must count as expired"
        );
    }

    #[test]
    fn a_token_without_an_expiry_never_expires() {
        let clock = FakeClock::new(NOW);
        clock.advance(Duration::days(400));

        assert!(!token_set(None, None).expires_within(&clock, Duration::seconds(60)));
    }

    #[test]
    fn scopes_are_split_on_whitespace() {
        assert_eq!(token_set(None, None).scopes, vec!["repo", "read:org"]);
    }

    #[test]
    fn refreshing_keeps_the_old_refresh_token_when_the_provider_omits_it() {
        let original = token_set(Some(60), Some("refresh-1"));
        let refreshed = TokenResponse {
            access_token: "access-2".to_owned(),
            refresh_token: None,
            token_type: None,
            expires_in: Some(3600),
            scope: None,
        }
        .into_token_set(NOW);

        let merged = original.merge_refreshed(refreshed);

        assert_eq!(merged.access_token.expose(), "access-2");
        assert_eq!(
            merged.refresh_token.as_ref().map(|t| t.expose()),
            Some("refresh-1"),
            "dropping the refresh token here would log the user out on the next refresh"
        );
        assert_eq!(merged.scopes, vec!["repo", "read:org"]);
    }

    #[test]
    fn a_rotated_refresh_token_replaces_the_old_one() {
        let original = token_set(Some(60), Some("refresh-1"));
        let refreshed = TokenResponse {
            access_token: "access-2".to_owned(),
            refresh_token: Some("refresh-2".to_owned()),
            token_type: None,
            expires_in: Some(3600),
            scope: None,
        }
        .into_token_set(NOW);

        let merged = original.merge_refreshed(refreshed);

        assert_eq!(
            merged.refresh_token.as_ref().map(|t| t.expose()),
            Some("refresh-2")
        );
    }

    #[test]
    fn the_stored_shape_round_trips() {
        let tokens = token_set(Some(3600), Some("refresh-1"));
        let encoded = serde_json::to_string(&StoredTokenSet::from(&tokens)).unwrap();
        let decoded: TokenSet = serde_json::from_str::<StoredTokenSet>(&encoded)
            .unwrap()
            .into();

        assert_eq!(decoded.access_token.expose(), "access");
        assert_eq!(decoded.expires_at, tokens.expires_at);
        assert_eq!(decoded.scopes, tokens.scopes);
    }
}
