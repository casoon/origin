# origin-http

HTTP port for Origin: request and response types, rate-limit parsing, status mapping. Knows no HTTP library.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_http::{HttpClient, HttpRequest};
use time::OffsetDateTime;

async fn fetch_profile(
    client: &dyn HttpClient,
    token: &str,
) -> origin_domain::Result<serde_json::Value> {
    let request = HttpRequest::get("https://api.example.com/profile").bearer(token);

    let response = client.send(request).await?;
    let response = response.error_for_status(OffsetDateTime::now_utc())?;

    response.json::<serde_json::Value>()
}
```

`HttpClient` is a trait; connectors and SDK crates receive an implementation of it rather
than depending on a concrete HTTP library. For an offline test double, see
`origin_http::testing::MockHttpClient` (behind the `testing` feature).

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
