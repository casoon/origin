# origin-mcp-http

MCP over a local loopback HTTP endpoint for Origin applications (ADR-0031).

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_mcp_core::McpServer;
use origin_mcp_http::{HttpTransport, Token};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

let server = Arc::new(McpServer::new("my-product", "0.1.0"));
let token = Token::generate();
let cancellation = CancellationToken::new();

// Binds an ephemeral port on 127.0.0.1:0 and publishes discovery info
let transport = HttpTransport::bind(server, token, cancellation).await?;
let port = transport.port();
```

Enables external AI clients to connect to an already-running GUI application without
process restarts or single-instance collisions. Endpoint access is guarded by constant-time
bearer token authentication and runtime discovery files.

## Stability

Pre-1.0 (`0.2.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
