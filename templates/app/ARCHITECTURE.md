# Architecture

These rules are binding for this project. `cargo xtask validate` enforces what can be
checked mechanically, and it runs in CI.

They come from [Origin](https://github.com/casoon/origin); the reasoning behind each one
is in its `adr/` directory.

## The central rule

**Domain logic does not know Tauri exists.**

Tauri is the desktop *host*. The application is a set of independently testable Rust
components that depend on traits, never on an `AppHandle`.

```text
PRODUCT              src-tauri/src/lib.rs — the composition root
  ↓
DRIVING ADAPTERS     Tauri host · MCP server · CLI — all equal peers
  ↓
MODULES              your feature areas
  ↓
ORIGIN PLATFORM      storage · settings · secrets · events · sync · jobs
  ↓
PLATFORM CONTRACTS   the OS capabilities domain code may depend on
```

The quality gate that keeps this honest: **the application must be testable without
starting Tauri.** If a workflow can only be exercised through a running desktop session,
logic has leaked into the host or the UI.

## The rules

1. Domain code never depends on Tauri.
2. External APIs are isolated behind connectors or SDK crates.
3. OS access happens through platform contracts, never directly.
4. The application is assembled in one composition root. No hidden global state.
5. Cross-cutting reactions use typed events, never string topics.
6. External services remain the source of truth. Local storage is cache unless
   documented otherwise.
7. Credentials live in the OS keychain, never in the database.
8. Security follows least privilege: no blanket `fs:*` or `shell:*` grant.
9. Frontend views never call Tauri APIs directly — they go through `@origin/client`.
10. A command resolves state, delegates and translates errors. No logic lives in one.

## File ownership

| Role | Update behaviour |
| --- | --- |
| **origin-owned** | generated from `app.toml`; overwritten, never hand-edited |
| **shared** | Origin provides a baseline, you extend it through `app.toml` |
| **product-owned** | yours; Origin never touches it |

`src-tauri/capabilities/*.json` is generated. To change what a window may do, change its
profile in `app.toml` and run `cargo xtask generate`.

## Deviating

You may deviate from a recommendation. The deviation must be **explicit** — declared in
`app.toml`:

```toml
[origin.overrides]
hand_written_capabilities = true
```

An update then skips it and reports it as a manual step, instead of overwriting a
decision you made on purpose. Silent deviation is a bug.
