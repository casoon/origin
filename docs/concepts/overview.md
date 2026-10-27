---
title: Architecture overview
description: "The layers, which crate sits where, how modules plug into the platform, and how the Tauri host connects the application to the desktop."
order: 1
---

Origin splits a desktop application into layers that only depend downwards. Tauri is the
bottom layer, the *host*, and nothing above it knows it exists. The binding rules are in
[ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md);
[Architecture in the code](../architecture/) explains the reasoning.

## Layers

```text
PRODUCT APP            examples/demo, later other independent products
  ↓ composition root
DRIVING ADAPTERS       Tauri host · MCP server · CLI · headless — all equal peers
APPLICATION MODULES    ApplicationModule implementations
  ↓
CONNECTORS             external service integrations
  ↓
ORIGIN PLATFORM        events, secrets, settings, storage, http, auth, accounts,
                       connector, sync, jobs, mcp-core, ai, telemetry, app
TOOLING                manifest, xtask — build time, not runtime
  ↓
PLATFORM CONTRACTS     origin-platform, origin-domain (ports)
  ↓
TAURI HOST             host/origin-tauri, adapters/origin-*-tauri
```

## Where each crate sits

| Folder | Crates | Role |
| --- | --- | --- |
| `crates/` | `origin-domain`, `origin-platform` | error model, domain primitives and the OS contracts (ports) |
| `crates/` | `origin-events`, `origin-secrets`, `origin-settings`, `origin-storage`, `origin-http`, `origin-auth`, `origin-accounts`, `origin-connector`, `origin-sync`, `origin-jobs`, `origin-mcp-core`, `origin-ai`, `origin-telemetry` | platform mechanisms, each behind a trait |
| `crates/` | `origin-app` | `ApplicationBuilder`, modules, service registry |
| `crates/` | `origin-manifest`, `origin-xtask` | `app.toml`, code generation, architecture checks |
| `adapters/` | `origin-storage-sqlite`, `origin-secrets-system`, `origin-http-reqwest`, `origin-auth-loopback`, `origin-mcp-stdio`, `origin-mcp-http`, `origin-process-std`, `origin-workspace-fs`, `origin-workspace-watch`, `origin-notifications-tauri` | concrete implementations of the contracts |
| `host/` | `origin-tauri` | plugin wiring, tray, IPC commands, event bridge |
| `frontend/` | `@casoon/origin-client`, `@casoon/origin-ui` | the typed IPC client and shared Svelte 5 components |

The [crate reference](../../reference/crates/) links each one to crates.io and docs.rs.

`cargo xtask validate` checks the dependency direction mechanically:

| Layer | May depend on | Must never depend on |
| --- | --- | --- |
| `crates/*` | other `crates/*`, third-party libs | `tauri*`, `adapters/*`, `host/*`, `examples/*` |
| `adapters/*` | `crates/*`, platform libs | other `adapters/*`, `examples/*` |
| `host/*` | `crates/*`, `adapters/*`, `tauri*` | `examples/*` |
| `examples/*` | everything | — |

## Modules and the platform

A module is a feature area compiled into the product. It implements one trait from
`origin-app`:

```rust
pub trait ApplicationModule: fmt::Debug + Send + Sync + 'static {
    fn id(&self) -> &'static str;
    fn register(&self, registry: &mut ModuleRegistry) -> Result<()>;
}
```

In `register`, a module reads the `Platform` (`registry.platform()`), provides its
services (`registry.provide(Arc::new(...))`) and subscribes to events. Other code
resolves those services by type with `require::<T>()`. There is no dynamic plugin
loading.

`Platform` is what every module may rely on. Some services are always present; the
others exist only when the composition root wired them, so a missing capability is
absent rather than switched off:

| Always present | Only when the product wires it |
| --- | --- |
| `clock`, `events`, `storage`, `cache`, `secrets`, `settings`, `notifications`, `confirmation`, `jobs`, `sync`, `accounts`, `connectors` | `tray`, `opener`, `http`, `workspace_fs`, `workspace_watcher`, `process_runner`, `global_shortcuts` |

Optional services are read through methods such as `platform.http()` or
`platform.opener()`, which return a configuration error naming what is missing instead
of panicking.

## The Tauri integration

`origin-tauri` is the only crate that turns an `Application` into a desktop program. A
product's `main.rs` uses three entry points:

```rust
fn main() {
    let config = HostConfig::new("dev.origin.demo");
    origin_tauri::builder(&config)
        .invoke_handler(origin_handler![my_product_command])
        .setup(move |app| {
            let application = my_product::build(app.handle())?;
            origin_tauri::attach(app.handle(), application, &config)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to start");
}
```

**`builder(&config)`** returns a `tauri::Builder` with the Origin plugin set:
single-instance first (a second launch focuses the main window), then notifications and
the opener, then window state. Single-instance and window state follow `HostConfig`;
both are on by default.

**`attach(app, application, &config)`** hands the assembled application to the shell.
It:

- wraps the application in `OriginState` and registers it as Tauri-managed state,
- starts the sync engine on Tauri's async runtime, stoppable through
  `OriginState::shutdown`,
- forwards every `PlatformEvent` from the in-process bus to the webview on
  `origin://platform-event` — one way only; the frontend observes, and anything it
  wants to cause goes through a command,
- installs the tray (Show window, Quit) when the product enabled it.

**`origin_handler![...]`** builds the single invoke handler Tauri allows: Origin's own
commands plus the product's. Calling `tauri::generate_handler!` directly would drop the
platform commands. The platform commands are:

| Area | Commands |
| --- | --- |
| App | `origin_app_info`, `origin_health` |
| Settings | `origin_setting_get`, `origin_setting_set`, `origin_settings_customised` |
| Opener | `origin_open_url` |
| Accounts and connectors | `origin_accounts`, `origin_account_disconnect`, `origin_connectors` |
| Jobs | `origin_jobs`, `origin_job_cancel` |
| Sync | `origin_sync_status`, `origin_sync_now` |

Commands resolve state, call the domain and translate errors. A failure reaches the
frontend as `CommandError`, a transparent wrapper around the stable `ErrorContract`,
never as a raw `rusqlite`, `reqwest` or `tauri` error.

### Default wiring

`origin_tauri::defaults` holds the standard adapter constructions a composition root
calls instead of repeating them. Each one can be replaced individually:

| Function | Provides |
| --- | --- |
| `storage` | SQLite in the platform data directory for the app id (`origin.sqlite3`), resolved through `origin_platform::paths` so a headless run reaches the same file |
| `secret_store` | the OS credential store, scoped to the app id |
| `notifications` | native notifications through `tauri-plugin-notification` |
| `http_client` | one reqwest client with a `name/version (app id)` user agent |
| `opener` | `TauriOpener`, which accepts only `http://` and `https://` URLs |

### The frontend side

`@casoon/origin-client` is the only package that imports `@tauri-apps/api`. Views call
typed functions (`appInfo()`, `settings.get()`, …) and subscribe with
`onPlatformEvent`; `cargo xtask validate` fails on a direct Tauri import anywhere else.
The types crossing the boundary are generated from their Rust definitions, see
[The manifest](../manifest/).

## Driving the core without Tauri

The host is one driving adapter among several. The same `Application` answers an MCP
client over stdio (`origin-mcp-stdio`) or authenticated loopback HTTP
(`origin-mcp-http`), and the whole application is testable with
`ApplicationBuilder::in_memory()` and no desktop session — see
[AI integration](../../guides/ai-integration/) and [Testing](../../guides/testing/).
