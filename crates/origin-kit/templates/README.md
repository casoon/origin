# __PRODUCT_NAME__

A desktop application built on [Origin](https://github.com/casoon/origin) — Rust, Tauri
and Svelte.

## What you already have

This is not an empty window. The scaffold ships with:

| | |
| --- | --- |
| **Architecture** | domain logic in Rust, testable without starting Tauri |
| **Errors** | one normalised model; the frontend switches on a kind, never on a message |
| **Storage** | SQLite for cache, read models and settings |
| **Secrets** | the OS keychain — never the database |
| **Settings** | typed, with key and default declared together |
| **Logging** | structured tracing with correlation fields |
| **Security** | a named capability profile per window; no blanket filesystem or shell access |
| **CI** | architecture rules, generated-file checks, tests on three platforms |
| **Release** | a tag-driven workflow that builds for macOS, Windows and Linux |

## Requirements

| Tool | Version |
| --- | --- |
| Rust | 1.88 or newer |
| Node | 22 or newer |
| pnpm | 10 or newer |
| Tauri CLI | 2.x — `cargo install tauri-cli --version "^2"` |

On Linux, also install `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`,
`libayatana-appindicator3-dev`, `librsvg2-dev` and `libsoup-3.0-dev`.

## Running it

```bash
pnpm install
cargo tauri dev
```

## Layout

```text
app.toml            what this product is — generated config comes from here
src-tauri/          the Rust application
  src/lib.rs          the composition root: every dependency in one function
  src/example.rs      a feature area; replace it with your own
  src/commands.rs     the IPC surface
  capabilities/       generated — do not edit
ui/                 the Svelte frontend
  src/client.ts       typed wrappers around commands
xtask/              three lines; the tasks live in origin-xtask
docs/               how to work on this project
```

## Everyday commands

```bash
cargo tauri dev              # run the app
cargo test --workspace       # Rust tests — no desktop session required
pnpm check                   # TypeScript and Svelte checks

cargo xtask validate         # enforce the architecture rules
cargo xtask generate         # write the files derived from app.toml
cargo xtask generate --check # fail if generated files are stale or hand-edited
cargo xtask update           # adopt a newer Origin version
cargo xtask ci               # everything CI runs
```

## Where to go next

- [docs/getting-started.md](docs/getting-started.md) — the tour, in reading order
- [docs/adding-a-feature.md](docs/adding-a-feature.md) — module, command, client, test
- [docs/security.md](docs/security.md) — capabilities, secrets, what not to grant
- [docs/releasing.md](docs/releasing.md) — signing, notarisation, updates
- [docs/updating-origin.md](docs/updating-origin.md) — staying current
- [ARCHITECTURE.md](ARCHITECTURE.md) — the rules this project is held to

## Before you ship

- Replace the placeholder icons in `src-tauri/icons` (`cargo tauri icon your-icon.png`).
- Delete the example module once you have a real feature.
- Read [docs/releasing.md](docs/releasing.md) — until you configure signing, builds are
  unsigned, which is fine for development and not fine for users.
