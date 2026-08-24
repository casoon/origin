# Authentication and connectors

Decisions: [ADR-0006](../adr/0006-connector-architecture.md),
[ADR-0014](../adr/0014-http-as-a-port.md),
[ADR-0015](../adr/0015-oauth-redirect-via-loopback.md),
[ADR-0016](../adr/0016-multi-account-from-day-one.md).

## The flow

```text
LoopbackRedirect::bind()      → http://127.0.0.1:<ephemeral>/callback
AuthorizationFlow::begin()    → authorization url + state + PKCE verifier
Opener::open_url()            → the user consents in their own browser
RedirectListener::wait()      → code, after verifying `state`
AuthorizationFlow::exchange() → TokenSet
AccountService::connect()     → account record + credentials in the keychain
```

In code:

```rust
let listener = LoopbackRedirect::bind().await?;
let tokens = flow.authorize(&listener, opener.as_ref()).await?;
let account = platform.accounts.connect(&connector_id, "work", &tokens).await?;
```

Afterwards nothing calls the flow again. Connectors ask
[`AccessTokenProvider`] for a token and get a valid one:

```rust
let token = tokens.access_token(&account_id).await?;
let request = HttpRequest::get(url).bearer(token.expose());
```

## What the flow guarantees

- **PKCE (S256) always**, including for confidential clients. `plain` is not supported.
- **`state` is verified before the code is used.** A redirect the application did not
  start is rejected, and no token request is sent for it.
- **The verifier never appears in a URL** the user or their browser history can see.
- **Loopback, not a custom URL scheme** — same behaviour on all three platforms, no OS
  registration, and no other application can claim the redirect.
- **The listener times out.** A user who closes the browser tab does not leave a
  listener — and whatever awaits it — alive for the rest of the session.

## Refresh

`AccessTokenProvider` refreshes 60 seconds before expiry, because a token that is valid
for another two seconds when checked will be rejected by the time the request lands.

Two details that are easy to get wrong and are covered by tests:

- **Refresh is single-flight.** Ten concurrent requests on an expiring token trigger one
  refresh. With a provider that rotates refresh tokens, the naive version invalidates
  its own credentials.
- **A refresh response without a `refresh_token` keeps the old one.** Many providers
  omit it, meaning "keep using what you have". Dropping it logs the user out on the
  next refresh.

A refresh token the provider *rejects* is discarded immediately — retrying it forever
would never succeed and would keep the account looking connected.

## Accounts

An account is required, not optional, on anything that reaches an external service.
A connector may have several (work and personal GitHub, many Cloudflare zones).

- The account list lives in `Storage`; the credentials live in the OS credential store.
- Disconnecting removes the record and the credentials. It does **not** remove data a
  module cached for that account — only the module knows its namespaces.
- `mark_expired` sets the status and publishes `AccountExpired`; it keeps the
  credentials, because a provider outage looks the same as an expired token and would
  otherwise force a needless reconnect.

## HTTP and rate limits

Connectors depend on the `HttpClient` port, never on `reqwest`. That gives the
application one connection pool, one timeout policy, and offline connector tests:

```rust
let http = MockHttpClient::new();
http.push_json(200, r#"{"login":"octocat"}"#);
```

Rate-limit metadata is parsed once, in `origin-http`, from `RateLimit-*`,
`X-RateLimit-*` and `Retry-After`. Two subtleties are handled there rather than in each
connector:

- `reset` is a delta in the IETF draft and an absolute unix timestamp in the
  GitHub-style headers.
- A `403` with an exhausted budget is a **rate limit**, not a permission problem.
  Getting that wrong sends the user off to re-authenticate for no reason.

## Adding a connector

1. Implement `Connector`: `id`, `descriptor`, `verify`.
2. Declare the permissions the descriptor needs. Keep them read-only unless the product
   genuinely writes — `descriptor.requests_write_access()` is worth asserting on in a
   test.
3. Register it in the composition root: `.connector(GitHubConnector::new(...))`.
4. Put the service's own API surface in a context-free SDK crate (ADR-0003) that takes
   an `Arc<dyn HttpClient>` and knows nothing about Origin.

`verify` is the one operation every connector must support. It turns "we have a token"
into "we have a working account", and returning `AppError::Authentication` from it is
what tells the platform to mark the account expired instead of retrying.
