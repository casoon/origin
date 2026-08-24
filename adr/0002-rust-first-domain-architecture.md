# ADR-0002  Rust-first Domain Architecture

Status:   Accepted
Date:     2026-08-23

## Context

In many Tauri projects the real logic ends up in the frontend, with Rust reduced to a
thin IPC shim. That makes logic untestable, duplicated per platform, and dependent on
a browser runtime.

## Decision

Business logic lives in Rust. The frontend renders state and dispatches intents.

The quality gate: **the application must be testable without starting Tauri.**

```rust
let app = ApplicationBuilder::new()
    .storage(MemoryStorage::new())
    .secret_store(MemorySecretStore::new())
    .clock(FakeClock::new(...))
    .build()?;
```

## Consequences

- Frontend stays replaceable; a CLI or headless agent stays possible.
- Every port needs an in-memory test double. This is a feature, not overhead.
- Frontend developers cannot "just add a fetch call" — intended.
