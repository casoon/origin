# origin-auth-loopback

Local loopback redirect listener for OAuth authorization flows (ADR-0015, RFC 8252).

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_auth::RedirectListener;
use origin_auth_loopback::LoopbackRedirect;

// Binds an ephemeral port on 127.0.0.1:0
let listener = LoopbackRedirect::bind().await?;
let redirect_uri = listener.redirect_uri();

// Await the authorization callback from the browser
let code = listener.await_code().await?;
```

Binds an ephemeral port on `127.0.0.1`, captures exactly one OAuth redirect from the
system browser, renders a success response, and shuts down immediately without leaving
open sockets or requiring custom OS URL scheme registrations.

## Stability

Pre-1.0 (`0.2.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
