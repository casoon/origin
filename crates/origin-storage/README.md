# origin-storage

Storage port and TTL cache for Origin. Knows no storage engine.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_storage::{Cache, MemoryStorage, StorageKey};
use origin_domain::SystemClock;
use std::sync::Arc;
use time::Duration;

let cache = Cache::new(Arc::new(MemoryStorage::new()), Arc::new(SystemClock));
let key = StorageKey::new("github", "notifications");

cache.put(&key, &vec!["a", "b"], Some(Duration::minutes(5))).await?;

let value: Option<Vec<String>> = cache.get(&key).await?;
```

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
