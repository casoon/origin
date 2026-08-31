# origin-http-reqwest

HttpClient adapter backed by reqwest.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_http_reqwest::ReqwestHttpClient;
use std::time::Duration;

// Build one and keep it: `reqwest::Client` owns the connection pool, so constructing
// several defeats keep-alive.
let client = ReqwestHttpClient::builder("my-app/1.0")
    .timeout(Duration::from_secs(15))
    .max_response_bytes(5 * 1024 * 1024)
    .build()?;
```

`ReqwestHttpClient::new("my-app/1.0")` builds one with the default timeout, connect
timeout and 10 MiB response ceiling. Both implement `origin_http::HttpClient`.

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
