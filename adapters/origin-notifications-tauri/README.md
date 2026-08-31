# origin-notifications-tauri

NotificationService adapter backed by tauri-plugin-notification.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_notifications_tauri::TauriNotificationService;
use origin_platform::{Notification, NotificationService, Urgency};
use tauri::AppHandle;

fn setup<R: tauri::Runtime>(app: AppHandle<R>) {
    let notifications = TauriNotificationService::new(app);

    let notification = Notification::new("Sync complete")
        .with_body("3 items updated")
        .with_urgency(Urgency::Normal);

    // Requests OS notification permission on first use, if not already decided.
    tauri::async_runtime::spawn(async move {
        let _ = notifications.notify(notification).await;
    });
}
```

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
