# origin-mcp

Model Context Protocol (MCP) server boundary for Origin applications.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_mcp::{AiPermission, McpServer, Tool, ToolDescriptor, ToolOutput};
use serde_json::Value;

struct StatusTool;

#[async_trait::async_trait]
impl Tool for StatusTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor::new(
            "app.status",
            "Application status",
            "Returns current system status",
            AiPermission::Read,
        )
    }

    async fn call(&self, _args: Value) -> origin_domain::Result<ToolOutput> {
        Ok(ToolOutput::text("healthy"))
    }
}

let mut server = McpServer::new("my-product", "0.1.0");
server.register(StatusTool);
```

MCP acts as a **driving adapter** (ADR-0027) allowing external AI clients to control
application services. Tools wrap domain services, never UI commands, and execution is
governed by granular permission levels (`AiPermission`).

## Stability

Pre-1.0 (`0.2.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
