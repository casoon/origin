# Testing

## The gate

**An Origin application must be testable without starting Tauri.** If a workflow can
only be exercised through a running desktop session, logic has leaked into the host or
the UI.

```rust
let application = ApplicationBuilder::in_memory()
    .clock(Arc::new(FakeClock::new(datetime!(2026-08-23 10:00 UTC))))
    .notifications(Arc::new(RecordingNotificationService::new()))
    .module(PulseModule)
    .build()?;
```

## Contract tests

Swappable implementations must behave identically, so the test suite lives with the
*contract*, not with each implementation:

```rust
#[tokio::test]
async fn satisfies_the_secret_store_contract() {
    origin_secrets::contract::run_all(&MySecretStore::new(), "my-store-test").await;
}
```

The same suite runs against `MemorySecretStore` and against the real system keychain.
The keychain run is `#[ignore]`d — it touches a developer's login keychain and can raise
a prompt, so it must never run unattended:

```bash
cargo test -p origin-secrets-system -- --ignored
```

`origin_storage::contract` works the same way, and covers what is easy to get subtly
wrong: namespace isolation, idempotent deletes, and expiry metadata surviving a round
trip.

## Test doubles

| Port | Double | Feature |
| --- | --- | --- |
| `Clock` | `FakeClock` | `origin-core/testing` |
| `NotificationService` | `RecordingNotificationService` | `origin-platform/testing` |
| `Opener` | `RecordingOpener` | `origin-platform/testing` |
| `SecretStore` | `MemorySecretStore` | always available |
| `Storage` | `MemoryStorage` | always available |

Doubles for the recording variants sit behind a `testing` feature so they are not
shipped in release builds.

## Time

Never call `OffsetDateTime::now_utc()` in domain code. Take a `Clock`. Then TTL, backoff
and scheduling can be tested by advancing a `FakeClock` rather than by sleeping.

## Architecture tests

`cargo xtask validate` enforces the mechanical part of `ARCHITECTURE.md`: layer
dependencies, product names in platform code, `@tauri-apps/api` imports outside
`@origin/client`, and blanket capability grants. It runs in CI as its own fast job.
