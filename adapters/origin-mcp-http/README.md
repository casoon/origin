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

// Binds 127.0.0.1:0 — the OS picks the port.
let transport = HttpTransport::bind(Token::generate()).await?;

// URL and token for clients, in a file only the current user can read.
transport.discovery().write(&discovery_path)?;

transport.serve(server, CancellationToken::new()).await?;
```

Enables external AI clients to connect to an already-running GUI application without
process restarts or single-instance collisions. Every request needs the bearer token
(compared in constant time); requests with a foreign `Host` or `Origin` are refused, which
keeps web pages and DNS rebinding out. Each `initialize` opens its own session
(`Mcp-Session-Id`), so several clients can attach at once.

## Stability

Pre-1.0 (`0.2.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
