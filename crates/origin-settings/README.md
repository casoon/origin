# origin-settings

Typed user settings for Origin, backed by any Storage implementation.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_settings::{Setting, Settings, StorageSettingsStore};
use origin_storage::MemoryStorage;
use std::sync::Arc;

const THEME: Setting<String> = Setting::new("ui.theme", || String::from("system"));

// `MemoryStorage` is an in-memory `Storage` implementation from `origin-storage`;
// swap in a persistent one in production. `clock` is an `Arc<dyn Clock>`.
let store = StorageSettingsStore::new(Arc::new(MemoryStorage::new()), clock);
let settings = Settings::new(Arc::new(store));

settings.set(&THEME, &"dark".to_string()).await?;
let theme = settings.get(&THEME).await?;
```

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
