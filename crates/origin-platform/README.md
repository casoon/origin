# origin-platform

Platform contracts for Origin: the OS capabilities domain code is allowed to depend on.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_platform::{Notification, NotificationService, NoopNotificationService, Urgency};

let notifications = NoopNotificationService;
notifications
    .notify(
        Notification::new("Sync finished")
            .with_body("42 records updated")
            .with_urgency(Urgency::Low),
    )
    .await?;
```

The `testing` feature adds `RecordingNotificationService` and `RecordingOpener`, test
doubles that capture what would have been shown or opened instead of touching the OS.

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
