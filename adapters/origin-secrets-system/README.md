# origin-secrets-system

SecretStore adapter backed by the operating system credential store.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_secrets::{Secret, SecretKey, SecretStore};
use origin_secrets_system::SystemSecretStore;

// `service_prefix` scopes credentials so two Origin apps on the same machine cannot
// read each other's secrets.
let store = SystemSecretStore::new("dev.example.app");

let key = SecretKey::new("connector.github", "access_token");
store.set(&key, Secret::new("token-value")).await?;
let token = store.get(&key).await?;
```

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
