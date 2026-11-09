# Getting started

```bash
pnpm install
cargo tauri dev
```

## Read these four files, in this order

**1. `app.toml`** — what this product *is*. Its identity, which platform features it
uses, which modules are compiled in, and which security profile each window gets.
Everything derivable from it is generated rather than written by hand.

**2. `src-tauri/src/lib.rs`** — the composition root. Every dependency the application
has is visible in one function:

```rust
ApplicationBuilder::new()
    .storage(defaults::storage(app, config)?)
    .secret_store(defaults::secret_store(config))
    .notifications(defaults::notifications(app))
    .module(ExampleModule)
    .build()
```

There is no service locator and no global state. A missing dependency is a build error,
not a runtime surprise.

**3. `src-tauri/src/example.rs`** — a feature area. Note what it *cannot* do: it has no
database handle, no `AppHandle`, and no way to show a notification other than asking the
platform. That is what keeps it testable.

**4. The tests at the bottom of the same file** — the feature is exercised without
starting Tauri. That is the quality gate; if you ever cannot write such a test, logic
has leaked into the host or the UI.

## How a request flows

```text
Svelte component
  → ui/src/client.ts        a typed function, never invoke()
  → src-tauri/src/commands  resolve, delegate, translate errors
  → your service            the actual logic
  → the platform            storage · settings · secrets · events
```

Each layer exists for a reason. The client wrapper means the transport can change
without touching components. The thin command means the same logic is reachable from a
CLI, a headless run or an AI client.

## Everyday commands

```bash
cargo tauri dev              # run the app
cargo test --workspace       # Rust tests — no desktop session required
pnpm check                   # TypeScript and Svelte checks
cargo xtask validate         # enforce the architecture rules
cargo xtask generate         # write the files derived from app.toml
cargo xtask ci               # everything CI runs
```

## Two things people trip over

**Generated files.** `src-tauri/capabilities/*.json` comes from `app.toml`. Editing it
directly works until the next `generate` wipes it — and CI fails before that.

**The frontend build comes first.** `tauri::generate_context!` reads the built frontend
at compile time, so `pnpm build` has to run before a release build of the Rust side.
`cargo tauri dev` handles this for you.
