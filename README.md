# Origin

**A reference architecture and starter system for modular desktop applications built with Rust and Tauri.**

Origin is not a framework that replaces Tauri, and not a monolithic crate every
application must depend on. It is an opinionated set of architecture rules, reusable
platform crates, security conventions and build processes — plus a reference
application that demonstrates all of it.

> A new desktop application should not have to reinvent authentication, storage,
> synchronisation, permissions, updates, notifications, logging and project structure
> every single time.

**Independent where useful. Shared where proven. Explicit where different.**

## Status

Early. Implemented: the architecture contract, the platform contracts, the Tauri host
layer, OAuth with PKCE, account management, the connector contract, the sync engine,
background jobs, the app manifest with generated capabilities, and a running reference
application.

Distribution is prepared in the template: a tag-driven release workflow that builds
unsigned by default, states in the log what the artifact is, and turns signing on one
secret at a time. Nothing there is verified end to end — there is no product to sign yet.

## The central rule

**Domain code does not know Tauri exists.**

Tauri is the desktop *host*. The application itself is a set of independently testable
Rust components that depend on traits, not on an `AppHandle`.

```text
PRODUCT APP            examples/demo — later other independent products
  ↓ composition root
APPLICATION MODULES    feature areas, registered at compile time
  ↓
CONNECTORS             external service integrations
  ↓
ORIGIN PLATFORM        events · secrets · settings · storage · telemetry · app
  ↓
PLATFORM CONTRACTS     the OS capabilities domain code may depend on
  ↓
TAURI HOST             plugins, tray, IPC, capabilities
```

The quality gate that keeps this honest: **the whole application must be testable
without starting Tauri.**

```rust
let application = ApplicationBuilder::in_memory()
    .clock(Arc::new(FakeClock::new(now)))
    .notifications(Arc::new(RecordingNotificationService::new()))
    .module(PulseModule)
    .build()?;
```

The full rule set lives in [ARCHITECTURE.md](ARCHITECTURE.md); the reasoning behind each
decision lives in [adr/](adr/).

## Repository layout

```text
crates/            platform crates — never depend on Tauri, never know a product
  origin-domain      error model, domain primitives, Clock port
  origin-events      typed event bus
  origin-platform    notification and opener contracts
  origin-secrets     SecretStore contract + shared contract test suite
  origin-settings    typed settings
  origin-storage     Storage port + TTL cache
  origin-http        HttpClient port, rate limits, status mapping
  origin-auth        OAuth 2.0 + PKCE, token storage and refresh
  origin-accounts    several accounts per connector
  origin-connector   the connector contract
  origin-sync        sync engine: policies, backoff, offline, health
  origin-jobs        background jobs: progress, cancellation
  origin-mcp         the MCP boundary: tools an external AI may invoke
  origin-ai          inference the application performs itself
  origin-manifest    app.toml: what a product is, plus the security profiles
  origin-xtask       the maintenance tasks, as a library
  origin-telemetry   tracing setup and logging conventions
  origin-app         ApplicationBuilder, modules, service registry

adapters/          concrete implementations of the contracts
  origin-storage-sqlite      SQLite
  origin-secrets-system      Keychain / Credential Manager / Secret Service
  origin-notifications-tauri native notifications
  origin-http-reqwest        HTTP via reqwest
  origin-auth-loopback       RFC 8252 loopback redirect listener
  origin-mcp-stdio           MCP over stdio

host/origin-tauri  plugin wiring, tray, IPC commands, event bridge
frontend/client    @origin/client — the only package that speaks Tauri IPC
frontend/ui        @origin/ui — shared Svelte 5 components and design tokens
examples/demo      the reference application
templates/app      the project template `cargo xtask new` instantiates,
                   with its own English documentation set
xtask              entry point; the tasks live in crates/origin-xtask
adr/               architecture decision records
```

## Requirements

| Tool | Version |
| --- | --- |
| Rust | 1.88 or newer |
| Node | 22 or newer |
| pnpm | 10 or newer |
| Tauri CLI | 2.x (`cargo install tauri-cli --version "^2"`) |

On Linux you also need the Tauri system dependencies (`libwebkit2gtk-4.1-dev`,
`libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`).

> **Note**: If you use [Volta](https://volta.sh/) as your Node version manager on macOS, ensure your user owns `~/.volta` (`chown -R $(whoami) ~/.volta`) to prevent Volta permission errors during `pnpm` workspace checks.

## Getting started

```bash
pnpm install
cargo xtask demo
```

That builds and runs the reference application: a tray app with a background loop,
cached read model, typed events, native notifications and a Svelte 5 frontend that
never calls `invoke` directly.

Other tasks:

```bash
cargo xtask validate   # enforce the architecture rules
cargo xtask ci         # fmt + clippy + test + validate
cargo test --workspace # Rust tests, no desktop session required
pnpm -r check          # TypeScript and Svelte checks
```

The system-keychain contract test is excluded by default because it touches your real
login keychain. Run it deliberately:

```bash
cargo test -p origin-secrets-system -- --ignored
```

## What Origin gives an application

- **One error model.** Adapters translate `rusqlite`, `reqwest` and `tauri` errors into
  `AppError` at the boundary; the frontend receives a stable, classified contract and
  can branch on `kind` instead of parsing messages.
- **Contract tests.** Every swappable implementation passes the same suite, so the
  in-memory double and the real system keychain cannot drift apart.
- **Typed events.** `bus.subscribe::<PlatformEvent>()`, not `bus.on("sync:done")`. A
  renamed field is a compile error.
- **Least privilege.** Capabilities are scoped per window as named security profiles.
  Notifications and URL opening happen in Rust, so the frontend needs no permission for
  either. `cargo xtask validate` fails the build on a blanket `fs:*` or `shell:*` grant.
- **OAuth that does not cut corners.** PKCE always, `state` verified before the code is
  used, loopback redirect (no custom URL scheme), single-flight refresh, and a refresh
  response without a `refresh_token` keeps the old one instead of logging the user out.
- **Scheduling that belongs to the platform.** A connector says *how* to fetch; the
  engine owns *when* — retry, exponential backoff with jitter, offline handling,
  validators and single-flight. Scheduling is tested by moving a fake clock, not by
  sleeping.
- **Credentials in the OS keychain**, never in the application database, addressed per
  account so revoking one never touches another. Disconnecting an account clears
  everything stored under it in one call, by namespace convention.
- **A composition root that documents the product** — every dependency an application
  has is visible in one function.
- **Bring your own AI client, not your own API key.** MCP makes the application
  controllable by the AI the user already has — with a permission level of its own,
  granting read and propose but never commit or delete, because the caller is a model
  acting on content that may be hostile. Inference the application performs itself is a
  separate, swappable port.
- **Contracts generated, not mirrored.** Platform IPC types and each product's own
  command results are derived from their Rust definitions; a rename in Rust fails CI
  instead of surfacing as `undefined` in production.
- **An upgrade path, not a one-time copy.** Each project records the Origin version it
  tracks; `cargo xtask update` runs the migrations between it and the current one,
  regenerates, and hands back a checklist for anything only a human can decide. The
  migrations are tested against frozen fixture projects, and Origin's CI scaffolds a
  project from its own template on every pull request.
- **A manifest instead of copied config.** `app.toml` says what the product is;
  capability files are generated from it, so a security profile is a named choice rather
  than a permission list somebody widens one line at a time. The tasks themselves live
  in `origin-xtask` as a library — a derivative's `xtask` is three lines, and a new
  architecture rule reaches it with a version bump.

## Design principles

`Security first` · `Testability first` · `Replaceability first` · `Explicit architecture`
· `Minimal hidden magic` · `Small reusable components` · `Context-free libraries`

Origin deliberately avoids god objects, global app state, direct Tauri access from
domain code, string-based event systems, tokens in SQLite, API-specific errors in the
UI, and premature universal abstractions.

## Contributing

Read [ARCHITECTURE.md](ARCHITECTURE.md) first — it is binding. New abstractions follow
the Promotion Rule ([ADR-0009](adr/0009-module-promotion-rule.md)): a feature starts
where it is needed and moves into the platform only once a genuinely neutral shape has
emerged, in practice at the third occurrence. Deviating from a recommendation is
allowed, but the deviation must be explicit — in the app manifest or as an ADR.

## License

Origin is released under the [MIT License](LICENSE). Individual crates are intended to
stay usable on their own, outside Origin — in CLIs, services, automations and other
projects. No lock-in.
