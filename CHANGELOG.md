# Changelog

All notable changes to Origin are listed here. Origin follows
[Semantic Versioning](https://semver.org/): while it is 0.x, a minor version may change the
public API. All `origin-*` crates and both npm packages share one version and are released
together.

## [Unreleased]

## [0.2.1] - 2026-09-19

### Added

- **Client:** `eventChannel<Events>()` binds an event map once and returns a subscribe
  function whose handler payload narrows to the named event; `onEvent` stays for callers
  without a map. Details:
  [The frontend boundary](https://casoon.github.io/origin/docs/concepts/architecture/#the-frontend-boundary).
- **Docs:** project page with the HTML documentation at
  [casoon.github.io/origin](https://casoon.github.io/origin/).

### Fixed

- **Platform:** `origin-workspace-watch` reports macOS event paths relative to the
  workspace root (FSEvents reports `/private/var` for `/var`) and ignores events for the
  root itself.
- **Platform:** the shared workspace and process contract suites use platform-absolute
  roots, so they pass on Windows.

### Security

- **Dependencies:** the workspace lockfile moves `rustls` to 0.23.45 (RUSTSEC-2026-0285).

## [0.2.0] - 2026-09-11

### Added

- **Publishing:** all 29 crates are on crates.io, including the eight optional ones; the MCP
  core is published as `origin-mcp-core`. Details:
  [Publishing](https://casoon.github.io/origin/docs/lifecycle/publishing/#why-these-29-in-this-order).
- **npm:** the frontend packages are published as `@casoon/origin-client` and
  `@casoon/origin-ui`.
- **Platform:** contracts for workspace access, workspace watching, process execution,
  global shortcuts, user confirmation and the tray, each with an in-memory implementation
  and a shared contract test suite. Details:
  [Platform contracts](https://casoon.github.io/origin/docs/concepts/architecture/#platform-contracts).
- **Adapters:** `origin-workspace-fs`, `origin-workspace-watch` and `origin-process-std`
  implement the new workspace and process contracts.
- **MCP:** `origin-mcp-http` serves MCP over authenticated loopback HTTP and lets the stdio
  entry point proxy to a running instance. Details:
  [HTTP loopback](https://casoon.github.io/origin/docs/guides/ai-integration/#http-loopback--running-instance-attach).
- **Auth:** `AccountService::connect_with_token` connects accounts with a static token or
  personal access token. Details:
  [Static tokens and PATs](https://casoon.github.io/origin/docs/guides/authentication/#static-tokens-and-pats).
- **Manifest:** security profiles and a process execution allowlist. Details:
  [Security profiles](https://casoon.github.io/origin/docs/concepts/manifest/#security-profiles).

### Changed

- **MCP:** commit and delete tools need an explicit grant and a human confirmation; a
  missing or failed confirmation denies the call. Details:
  [Permissions](https://casoon.github.io/origin/docs/guides/ai-integration/#permissions).

## [0.1.0] - 2026-09-06

### Added

- **Platform:** first release of the 21 crates the application template depends on: error
  model, typed event bus, settings, storage, secrets, HTTP, telemetry, connector contract,
  composition root and the Tauri host. Details:
  [Crates](https://casoon.github.io/origin/docs/reference/crates/).
- **Auth:** OAuth with PKCE (https endpoints only) and account management with credential
  rollback. Details:
  [Authentication](https://casoon.github.io/origin/docs/guides/authentication/).
- **Sync:** sync engine with per-target single-flight runs, throttling and health. Details:
  [Sync and jobs](https://casoon.github.io/origin/docs/guides/sync-and-jobs/).
- **Jobs:** background jobs with progress and cancellation, plus `spawn_exclusive` and
  `spawn_awaitable` for one-at-a-time work and typed results. Details:
  [Jobs](https://casoon.github.io/origin/docs/guides/sync-and-jobs/#jobs).
- **MCP:** MCP server over stdio with an enforced initialize handshake and a read/propose
  permission level. Details:
  [AI integration](https://casoon.github.io/origin/docs/guides/ai-integration/).
- **xtask:** `cargo xtask` scaffolds products, generates capabilities from the app manifest
  and validates the architecture rules. Details:
  [App manifest](https://casoon.github.io/origin/docs/concepts/manifest/).

[Unreleased]: https://github.com/casoon/origin/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/casoon/origin/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/casoon/origin/tree/v0.2.0
[0.1.0]: https://crates.io/crates/origin-app/0.1.0
