---
title: The manifest
description: "app.toml says what a product is. Capability files and IPC types are generated, not hand-written."
order: 4
---

Decisions: [ADR-0021](https://github.com/casoon/origin/blob/main/adr/0021-manifest-is-the-single-source.md),
[ADR-0022](https://github.com/casoon/origin/blob/main/adr/0022-generated-files-are-origin-owned.md).

## Two questions, two places

```text
app.toml                 what is this product?
composition root         how is it assembled?
```

Anything derivable from the first is generated rather than written by hand. Generated
files can be updated without a merge, and that is what decides whether upgrading Origin
in four projects costs an afternoon or a week.

```toml
[origin]
version = "0.1.0"

[product]
id = "dev.origin.demo"
name = "Origin Demo"
version = "0.1.0"

[platform]
tray = true
notifications = true
single_instance = true
window_state = true

[modules]
pulse = true

[security.windows]
main = { profile = "standard-dashboard" }
```

The format is deliberately small. Every field becomes migration-liable the moment a
second product exists, so a field is added only when something is actually derived
from it.

## What gets generated

- `frontend/client/src/generated.ts` — every platform type that crosses the IPC boundary;
  product modules keep their own `ts-rs`-derived bindings next to their frontend client,
  derived from the Rust definitions (ADR-0024)
- `src-tauri/capabilities/*.json` — one file per security profile

```bash
cargo xtask generate          # write them
cargo xtask generate --check  # fail if stale or hand-edited (runs in CI)
```

Those commands own the platform bindings and capabilities. A product binding is derived
in its product crate and compared with the checked-in TypeScript during `cargo test`;
the demo's `PulseSnapshot` is the reference pattern.

Nothing in `@casoon/origin-client` hand-mirrors a Rust type any more: `types.ts` is a re-export
list. Adding an event variant in Rust now breaks the frontend's exhaustive `switch` at
check time instead of silently rendering nothing.

`generate` only removes files it recognises as its own, by marker — a hand-written
capability keeps working until someone converts it. It writes only when content changed,
so an unchanged run does not trigger a rebuild.

## Security profiles

A window picks a *named profile*, not a permission list:

| Profile | For |
| --- | --- |
| `readonly-dashboard` | reads state, receives events |
| `standard-dashboard` | the usual main window |
| `account-settings` | manages accounts through commands |
| `local-workspace` | workspace window: reads files under user-confirmed roots and executes allowlisted programs via Origin commands |

No profile grants direct filesystem, shell or process access through Tauri plugins — a unit test asserts
exactly that. Windows using `local-workspace` interact with the local filesystem and external tools strictly
through Origin IPC commands and Rust ports (`WorkspaceFs`, `ProcessRunner`), which enforce symlink containment
and executable allowlists in Rust before reaching the OS. Permissions are listed explicitly rather than pulling in a
plugin's `default` set, because a plugin default grows when the plugin is updated and silently widens every
window that used it.

To change what a profile *means*, change it in `origin-manifest`. It then changes for
every Origin application at once and is reviewed once instead of per project.

## Process execution allowlist

Products that need to invoke external tools declare an auditable allowlist in `app.toml`:

```toml
[security.process]
allowed_programs = ["git", "code"]
```

Entry names are validated at manifest load time (paths with directory separators are rejected)
and enforced in Rust by `ProcessAllowlist` before execution reaches the operating system.

## Validation

The manifest is validated at parse time for things types cannot express:

- a product id that is not reverse-DNS
- a window without a security profile — and a product with no windows at all
- `tray = true` together with `single_instance = false`, which would mean a second tray
  icon

## Deliberate deviations

```toml
[origin.overrides]
custom_window_management = true
```

A migration skips what is listed here and reports it as a manual step, instead of
overwriting a decision someone made on purpose (§46).

## Why xtask is a library

`origin-xtask` holds the tasks; a derivative's `xtask/src/main.rs` is three lines:

```rust
fn main() -> std::process::ExitCode {
    origin_xtask::main()
}
```

So the architecture rules, the generator and the CI recipe arrive with a version bump
instead of being copied into each project and drifting apart. A new rule lands in every
derivative and fails its CI the same day.
