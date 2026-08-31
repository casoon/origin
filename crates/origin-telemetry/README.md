# origin-telemetry

Tracing setup and logging conventions for Origin.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_telemetry::{Format, TelemetryConfig, spans};

// A process that speaks a protocol on stdout (e.g. MCP over stdio) must log to
// stderr instead, or a single log line corrupts the stream.
let config = TelemetryConfig {
    format: Format::Json,
    ..TelemetryConfig::for_stdout_protocol()
};
origin_telemetry::init(config);

let span = spans::sync(&sync_id, &connector_id, &account_id);
let _entered = span.enter();
tracing::info!("sync started");
```

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
