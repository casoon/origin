# origin-storage-sqlite

SQLite-backed Storage adapter for Origin.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_storage::{Record, Storage, StorageKey};
use origin_storage_sqlite::SqliteStorage;
use time::OffsetDateTime;

// Or `SqliteStorage::in_memory()` for tests and `--no-persist` runs.
let storage = SqliteStorage::open("app.sqlite")?;

let key = StorageKey::new("cache.pulls", "42");
storage.put(&key, Record::new("{}", OffsetDateTime::now_utc())).await?;
let record = storage.get(&key).await?;
```

`SqliteStorage` holds cache, read models, sync metadata and settings — never
credentials.

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
