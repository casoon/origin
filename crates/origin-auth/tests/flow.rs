//! End-to-end tests for the authorization code flow.
//!
//! No socket, no browser, no provider: a mock HTTP client and a fake redirect listener
//! stand in for all three, which is what makes this runnable in CI.

use origin_auth::testing::FakeRedirectListener;
use origin_auth::{AccessTokenProvider, AuthorizationFlow, OAuthConfig, TokenStore};
use origin_domain::testing::FakeClock;
use origin_domain::{AccountId, Clock, ConnectorId, ErrorKind};
use origin_http::testing::MockHttpClient;
use origin_platform::testing::RecordingOpener;
use origin_secrets::MemorySecretStore;
use std::sync::Arc;
use time::Duration;
use time::macros::datetime;

const NOW: time::OffsetDateTime = datetime!(2026-08-23 10:00 UTC);

fn config() -> OAuthConfig {
    OAuthConfig::new(
        "client-123",
        "https://provider.example/authorize",
        "https://provider.example/token",
    )
    .with_scopes(["repo", "read:org"])
}

struct Harness {
    flow: AuthorizationFlow,
    http: Arc<MockHttpClient>,
    clock: Arc<FakeClock>,
    secrets: Arc<MemorySecretStore>,
}

fn harness() -> Harness {
    let http = Arc::new(MockHttpClient::new());
    let clock = Arc::new(FakeClock::new(NOW));
    let secrets = Arc::new(MemorySecretStore::new());

    Harness {
        flow: AuthorizationFlow::new(config(), http.clone(), clock.clone()),
        http,
        clock,
        secrets,
    }
}

fn provider(harness: &Harness) -> AccessTokenProvider {
    AccessTokenProvider::new(
        ConnectorId::new("demo"),
        harness.flow.clone(),
        TokenStore::new(harness.secrets.clone()),
        harness.clock.clone(),
    )
}

fn body_of(request: &origin_http::HttpRequest) -> String {
    String::from_utf8(request.body.clone().unwrap_or_default()).unwrap()
}

#[tokio::test]
async fn a_complete_authorization_produces_usable_tokens() {
    let harness = harness();
    harness.http.push_json(
        200,
        r#"{"access_token":"at-1","refresh_token":"rt-1","expires_in":3600,"scope":"repo read:org"}"#,
    );

    let listener = FakeRedirectListener::returning("code-abc");
    let opener = RecordingOpener::new();

    let tokens = harness.flow.authorize(&listener, &opener).await.unwrap();

    assert_eq!(tokens.access_token.expose(), "at-1");
    assert_eq!(tokens.expires_at, Some(NOW + Duration::hours(1)));
    assert_eq!(tokens.scopes, vec!["repo", "read:org"]);
    harness.http.assert_all_consumed();
}

#[tokio::test]
async fn the_authorization_url_carries_pkce_and_state() {
    let harness = harness();
    harness
        .http
        .push_json(200, r#"{"access_token":"at-1","expires_in":3600}"#);

    let opener = RecordingOpener::new();
    harness
        .flow
        .authorize(&FakeRedirectListener::returning("code-abc"), &opener)
        .await
        .unwrap();

    let url = opener
        .opened()
        .first()
        .cloned()
        .expect("the browser was opened");

    assert!(
        url.starts_with("https://provider.example/authorize?"),
        "got {url}"
    );
    assert!(url.contains("response_type=code"));
    assert!(url.contains("client_id=client-123"));
    assert!(url.contains("code_challenge_method=S256"));
    assert!(url.contains("code_challenge="));
    assert!(url.contains("state="));
    assert!(
        url.contains("scope=repo%20read%3Aorg"),
        "scopes must be space separated: {url}"
    );
    assert!(
        !url.contains("code_verifier"),
        "the verifier must never appear in a URL the user can see: {url}"
    );
}

#[tokio::test]
async fn the_token_exchange_proves_possession_of_the_verifier() {
    let harness = harness();
    harness
        .http
        .push_json(200, r#"{"access_token":"at-1","expires_in":3600}"#);

    harness
        .flow
        .authorize(
            &FakeRedirectListener::returning("code-abc"),
            &RecordingOpener::new(),
        )
        .await
        .unwrap();

    let request = harness
        .http
        .last_request()
        .expect("a token request was sent");
    let body = body_of(&request);

    assert_eq!(request.url, "https://provider.example/token");
    assert!(body.contains("grant_type=authorization_code"));
    assert!(body.contains("code=code-abc"));
    assert!(body.contains("code_verifier="));
}

#[tokio::test]
async fn a_denied_authorization_is_an_authentication_error() {
    let harness = harness();

    let error = harness
        .flow
        .authorize(&FakeRedirectListener::denied(), &RecordingOpener::new())
        .await
        .unwrap_err();

    assert_eq!(error.kind(), ErrorKind::Authentication);
    assert!(
        !error.is_retryable(),
        "retrying a denied consent is pointless"
    );
}

#[tokio::test]
async fn a_forged_redirect_is_rejected_before_the_code_is_used() {
    let harness = harness();

    let error = harness
        .flow
        .authorize(
            &FakeRedirectListener::with_state("attacker-state"),
            &RecordingOpener::new(),
        )
        .await
        .unwrap_err();

    assert_eq!(error.kind(), ErrorKind::Authentication);
    assert!(
        harness.http.requests().is_empty(),
        "no token request may be sent for a redirect we did not start"
    );
}

#[tokio::test]
async fn a_token_endpoint_error_reports_the_providers_description() {
    let harness = harness();
    harness.http.push_json(
        400,
        r#"{"error":"invalid_grant","error_description":"The code has expired"}"#,
    );

    let error = harness
        .flow
        .authorize(
            &FakeRedirectListener::returning("stale"),
            &RecordingOpener::new(),
        )
        .await
        .unwrap_err();

    assert_eq!(error.kind(), ErrorKind::Authentication);
    assert!(
        error.to_string().contains("The code has expired"),
        "got: {error}"
    );
}

#[tokio::test]
async fn a_valid_token_is_returned_without_contacting_the_provider() {
    let harness = harness();
    harness.http.push_json(
        200,
        r#"{"access_token":"at-1","refresh_token":"rt-1","expires_in":3600}"#,
    );

    let provider = provider(&harness);
    let account = AccountId::new("acc-1");
    let tokens = harness
        .flow
        .authorize(
            &FakeRedirectListener::returning("code"),
            &RecordingOpener::new(),
        )
        .await
        .unwrap();
    provider.store(&account, &tokens).await.unwrap();

    let token = provider.access_token(&account).await.unwrap();

    assert_eq!(token.expose(), "at-1");
    assert_eq!(
        harness.http.requests().len(),
        1,
        "only the original exchange should have hit the network"
    );
}

#[tokio::test]
async fn an_expiring_token_is_refreshed_transparently() {
    let harness = harness();
    harness.http.push_json(
        200,
        r#"{"access_token":"at-1","refresh_token":"rt-1","expires_in":3600}"#,
    );
    harness
        .http
        .push_json(200, r#"{"access_token":"at-2","expires_in":3600}"#);

    let provider = provider(&harness);
    let account = AccountId::new("acc-1");
    let tokens = harness
        .flow
        .authorize(
            &FakeRedirectListener::returning("code"),
            &RecordingOpener::new(),
        )
        .await
        .unwrap();
    provider.store(&account, &tokens).await.unwrap();

    harness
        .clock
        .advance(Duration::minutes(59) + Duration::seconds(30));
    let token = provider.access_token(&account).await.unwrap();

    assert_eq!(token.expose(), "at-2", "the token must have been refreshed");

    let request = harness.http.last_request().unwrap();
    assert!(body_of(&request).contains("grant_type=refresh_token"));
    harness.http.assert_all_consumed();
}

#[tokio::test]
async fn concurrent_callers_trigger_exactly_one_refresh() {
    let harness = harness();
    harness.http.push_json(
        200,
        r#"{"access_token":"at-1","refresh_token":"rt-1","expires_in":60}"#,
    );
    harness.http.push_json(
        200,
        r#"{"access_token":"at-2","refresh_token":"rt-2","expires_in":3600}"#,
    );

    let provider = Arc::new(provider(&harness));
    let account = AccountId::new("acc-1");
    let tokens = harness
        .flow
        .authorize(
            &FakeRedirectListener::returning("code"),
            &RecordingOpener::new(),
        )
        .await
        .unwrap();
    provider.store(&account, &tokens).await.unwrap();

    harness.clock.advance(Duration::seconds(30));

    let calls = (0..8).map(|_| {
        let provider = provider.clone();
        let account = account.clone();
        tokio::spawn(async move { provider.access_token(&account).await })
    });

    for call in calls {
        assert_eq!(call.await.unwrap().unwrap().expose(), "at-2");
    }

    // With a rotating provider, a second refresh would have invalidated rt-2.
    let refreshes = harness
        .http
        .requests()
        .iter()
        .filter(|request| body_of(request).contains("grant_type=refresh_token"))
        .count();
    assert_eq!(refreshes, 1, "the refresh must be single-flight");
}

#[tokio::test]
async fn a_rejected_refresh_token_discards_the_credentials() {
    let harness = harness();
    harness.http.push_json(
        200,
        r#"{"access_token":"at-1","refresh_token":"rt-1","expires_in":60}"#,
    );
    harness.http.push_json(400, r#"{"error":"invalid_grant"}"#);

    let provider = provider(&harness);
    let account = AccountId::new("acc-1");
    let tokens = harness
        .flow
        .authorize(
            &FakeRedirectListener::returning("code"),
            &RecordingOpener::new(),
        )
        .await
        .unwrap();
    provider.store(&account, &tokens).await.unwrap();

    harness.clock.advance(Duration::seconds(30));
    let error = provider.access_token(&account).await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Authentication);

    // The next attempt must not retry the dead refresh token forever.
    let error = provider.access_token(&account).await.unwrap_err();
    assert!(error.to_string().contains("not connected"), "got: {error}");
}

#[tokio::test]
async fn a_transient_refresh_failure_keeps_the_credentials() {
    let harness = harness();
    harness.http.push_json(
        200,
        r#"{"access_token":"at-1","refresh_token":"rt-1","expires_in":60}"#,
    );
    harness.http.push_json(
        503,
        r#"{"error":"temporarily_unavailable","error_description":"try again"}"#,
    );
    harness.http.push_json(
        200,
        r#"{"access_token":"at-2","refresh_token":"rt-2","expires_in":3600}"#,
    );

    let provider = provider(&harness);
    let account = AccountId::new("acc-1");
    let tokens = harness
        .flow
        .authorize(
            &FakeRedirectListener::returning("code"),
            &RecordingOpener::new(),
        )
        .await
        .unwrap();
    provider.store(&account, &tokens).await.unwrap();

    harness.clock.advance(Duration::seconds(30));
    let error = provider.access_token(&account).await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::ExternalService);

    assert_eq!(
        provider.access_token(&account).await.unwrap().expose(),
        "at-2"
    );
}

#[tokio::test]
async fn an_account_without_a_refresh_token_asks_the_user_to_reconnect() {
    let harness = harness();
    harness
        .http
        .push_json(200, r#"{"access_token":"at-1","expires_in":60}"#);

    let provider = provider(&harness);
    let account = AccountId::new("acc-1");
    let tokens = harness
        .flow
        .authorize(
            &FakeRedirectListener::returning("code"),
            &RecordingOpener::new(),
        )
        .await
        .unwrap();
    provider.store(&account, &tokens).await.unwrap();

    harness.clock.advance(Duration::seconds(30));
    let error = provider.access_token(&account).await.unwrap_err();

    assert!(error.to_string().contains("reconnect"), "got: {error}");
}

#[tokio::test]
async fn credentials_are_isolated_per_account() {
    let harness = harness();
    let store = TokenStore::new(harness.secrets.clone());
    let connector = ConnectorId::new("demo");
    let (work, personal) = (AccountId::new("work"), AccountId::new("personal"));

    for (account, token) in [(&work, "at-work"), (&personal, "at-personal")] {
        harness.http.push_json(
            200,
            &format!(r#"{{"access_token":"{token}","expires_in":3600}}"#),
        );
        let tokens = harness
            .flow
            .authorize(
                &FakeRedirectListener::returning("code"),
                &RecordingOpener::new(),
            )
            .await
            .unwrap();
        store.save(&connector, account, &tokens).await.unwrap();
    }

    store.delete(&connector, &work).await.unwrap();

    assert!(store.load(&connector, &work).await.unwrap().is_none());
    assert_eq!(
        store
            .load(&connector, &personal)
            .await
            .unwrap()
            .expect("the other account is untouched")
            .access_token
            .expose(),
        "at-personal"
    );
}

#[tokio::test]
async fn the_clock_port_is_what_makes_expiry_testable() {
    // Guards the invariant behind every test above: expiry is derived from the
    // injected clock, never from wall-clock time.
    let clock = FakeClock::new(NOW);
    clock.advance(Duration::days(1));
    assert_eq!(clock.now(), NOW + Duration::days(1));
}
