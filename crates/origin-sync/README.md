# origin-sync

The Origin sync engine: scheduling, backoff, offline handling and sync state. Knows no external service.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_sync::{SyncEngine, SyncPolicy, SyncTarget};
use std::sync::Arc;
use time::Duration;

let engine = SyncEngine::new(storage, clock, events);

let target = SyncTarget::new(connector_id, account_id, "notifications");
engine.register_every(target.clone(), Duration::minutes(1), Arc::new(source));

// Or with full control over backoff and offline retry:
// engine.register(target, SyncPolicy::every(Duration::minutes(1)), source);

let outcomes = engine.run_due(clock.now()).await;
```

`source` implements the `SyncSource` trait, deciding *how* one kind of data is
fetched; the engine decides *when*.

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
