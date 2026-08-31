# origin-auth

OAuth 2.0 authorization code flow with PKCE, token storage and refresh for Origin.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_auth::{AuthorizationFlow, OAuthConfig};

// `OAuthConfig::new` returns a `Result`: both endpoints must be `https://`.
let config = OAuthConfig::new(
    "client-id",
    "https://provider.example/authorize",
    "https://provider.example/token",
)?
.with_scopes(["profile", "offline_access"]);

// `http` is an `Arc<dyn HttpClient>` and `clock` an `Arc<dyn Clock>`, supplied by the
// composition root.
let flow = AuthorizationFlow::new(config, http, clock);

let pending = flow.begin(redirect_uri)?;
// Send the user to `pending.authorization_url`, then, once the redirect listener has
// the code:
let tokens = flow.exchange(&pending, &code).await?;
```

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
