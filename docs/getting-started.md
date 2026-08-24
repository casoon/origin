# Getting started

## Prerequisites

Rust 1.88+, Node 22+, pnpm 10+, and the Tauri CLI (`cargo install tauri-cli --version "^2"`).
On Linux, install the Tauri system dependencies listed in the [README](../README.md).

## Run the reference application

```bash
pnpm install
cargo xtask demo
```

The demo is a tray application with a background loop that produces a reading every
30 seconds, caches it, decides whether it is healthy, and raises an alert with a native
notification when it is not.

Things worth trying:

- **Close the window.** The app keeps running in the tray and keeps refreshing.
- **Press "Lower to 40 % and refresh"** a few times. The threshold is a setting stored
  in Rust; the next reading above it raises an alert — once, no matter how often it
  stays critical.
- **Watch "Last platform event".** That text comes from the typed event bus in Rust,
  forwarded to the webview by the host bridge.

## What to read next

The demo is roughly 400 lines of Rust. Read them in this order:

1. [`examples/demo/src-tauri/src/lib.rs`](../examples/demo/src-tauri/src/lib.rs) —
   the composition root. Every dependency the product has is in one function.
2. [`examples/demo/src-tauri/src/pulse.rs`](../examples/demo/src-tauri/src/pulse.rs) —
   the module. Note what it *cannot* do: it has no database handle, no `AppHandle`, and
   no way to show a notification other than asking the platform.
3. Its tests at the bottom of the same file — the whole feature is exercised without
   starting Tauri.

## Everyday commands

```bash
cargo xtask validate    # enforce ARCHITECTURE.md
cargo xtask ci          # fmt + clippy + test + validate
cargo test --workspace  # Rust tests
pnpm -r check           # TypeScript and Svelte checks
pnpm --filter @origin/demo build
```
