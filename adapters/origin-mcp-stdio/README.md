# origin-mcp-stdio

Standard I/O transport adapter for Model Context Protocol (MCP) servers in Origin.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_mcp::McpServer;
use origin_mcp_stdio::serve;

let server = McpServer::new("my-product", "0.1.0");

// Serve JSON-RPC over stdin/stdout until client disconnects
serve(&server).await?;
```

Serves headless MCP tool interactions over standard input and output streams when the GUI
is not running. To prevent protocol stream corruption, stdout is reserved strictly for protocol
messages (logging must be routed to stderr or disabled).

## Stability

Pre-1.0 (`0.2.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
