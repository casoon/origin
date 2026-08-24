# Architecture

The rules are in [ARCHITECTURE.md](../ARCHITECTURE.md). This page explains how they
play out in the code.

## Why Tauri is only the host

A service that holds an `AppHandle` can only run inside a Tauri process. That makes it
untestable without a desktop session and couples it to a Tauri major version. So domain
code depends on traits instead:

```text
NotificationService  (crates/origin-platform)
        ↓
TauriNotificationService  (adapters/origin-notifications-tauri)
        ↓
tauri-plugin-notification
```

A Tauri 2 → 3 migration touches `host/` and `adapters/*-tauri`, nothing else.

## The composition root

Every product has exactly one function that assembles it. There is no service locator,
no global state, and no runtime lookup by name — a missing dependency is a build error:

```rust
ApplicationBuilder::new()
    .storage(defaults::storage(app)?)
    .secret_store(defaults::secret_store(config))
    .notifications(defaults::notifications(app))
    .opener(defaults::opener(app))
    .module(PulseModule)
    .build()
```

Storage, credentials and notifications have no implicit default. Silently defaulting
storage would ship an application that loses data; silently defaulting credentials would
keep tokens in process memory. `ApplicationBuilder::in_memory()` provides all three at
once, and its name says exactly what you get.

## Modules

A module is a compile-time feature area. It registers services and subscribes to events:

```rust
impl ApplicationModule for PulseModule {
    fn id(&self) -> &'static str { "pulse" }

    fn register(&self, registry: &mut ModuleRegistry) -> Result<()> {
        registry.provide(Arc::new(PulseService::new(registry.platform().clone())));
        Ok(())
    }
}
```

There is no dynamic plugin loading. Everything in the binary got there by being linked
in, which keeps the dependency graph honest and the binary auditable.

## Events versus direct calls

- Need a result now? Call the service. `snapshot()`, `refresh()`, `store_secret()`.
- Might several independent components react? Publish a typed event.

The bus is keyed by type, so `subscribe::<PlatformEvent>()` is checked by the compiler.
Products publish their own enums on the same bus. Adding a variant breaks exhaustive
subscribers — on purpose.

## The frontend boundary

`@origin/client` is the only package that imports `@tauri-apps/api`. Everything else
calls typed functions:

```ts
import { settings } from "@origin/client";
await settings.set("demo.critical_above", 40);
```

`cargo xtask validate` fails the build if a component imports Tauri APIs directly.

## Storage and freshness

`Storage` is dumb persistence: it stores and returns records as given, expired or not.
`Cache` decides what "stale" means, using the injected `Clock`. Every backend therefore
agrees on expiry, and TTL behaviour is testable without sleeping.

External services stay Source of Truth. Deleting the local database costs a resync and
nothing else.
