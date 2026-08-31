# origin-connector

The connector contract for Origin: what an external service integration must declare and support.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_connector::{AuthKind, Connector, ConnectorDescriptor, ConnectorRegistry};
use origin_domain::ConnectorId;
use std::sync::Arc;

impl Connector for GitHubConnector {
    fn id(&self) -> ConnectorId {
        ConnectorId::new("github")
    }

    fn descriptor(&self) -> ConnectorDescriptor {
        ConnectorDescriptor::new(self.id(), "GitHub", AuthKind::OAuth2)
    }

    // `verify` omitted here; see the trait docs for its contract.
}

let mut registry = ConnectorRegistry::new();
registry.insert(Arc::new(GitHubConnector::default()));
```

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
