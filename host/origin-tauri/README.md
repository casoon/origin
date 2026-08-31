# origin-tauri

Tauri host layer for Origin: plugin wiring, tray, IPC commands and the event bridge.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_tauri::HostConfig;

let config = HostConfig::new("dev.example.app")
    .with_tray("Example")
    .with_close_to_tray(true);
```

`app_id` is a reverse-DNS product id; it also scopes credentials in the system keychain,
so two Origin apps on the same machine cannot read each other's tokens.

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
