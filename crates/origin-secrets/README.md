# origin-secrets

SecretStore contract for Origin, with an in-memory implementation and a shared contract test suite.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_secrets::{MemorySecretStore, Secret, SecretKey, SecretStore};

let store = MemorySecretStore::new();
let key = SecretKey::new("github", "access_token");

store.set(&key, Secret::new("ghp_example")).await?;

let value = store.get(&key).await?;
assert_eq!(value.as_ref().map(Secret::expose), Some("ghp_example"));
```

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
