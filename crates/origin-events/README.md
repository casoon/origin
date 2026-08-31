# origin-events

Typed, in-process event bus for Origin. No string topics, no untyped payloads.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_events::{EventBus, PlatformEvent, SyncCompleted};

let bus = EventBus::new();
let mut sync_events = bus.subscribe::<PlatformEvent>()?;

bus.publish(PlatformEvent::SyncCompleted(SyncCompleted {
    sync: sync_id,
    connector: connector_id,
    account: account_id,
    changed: 3,
    at: clock.now(),
}))?;

let event = sync_events.recv().await?;
```

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
