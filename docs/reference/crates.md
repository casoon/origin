---
title: Crates and packages
description: "Every published origin-* crate and npm package, grouped by layer, with links to crates.io, docs.rs and npm."
order: 1
---

All 29 crates and both npm packages share the workspace version, currently **0.2.0**.
Item-level API documentation lives on docs.rs; this page is the map. How the layers depend
on each other is described in the [architecture overview](../../concepts/overview/).

## Contracts

| Crate | Purpose | Links |
| --- | --- | --- |
| `origin-domain` | Origin domain primitives and platform ports. Knows no product, no Tauri, no storage engine. | [crates.io](https://crates.io/crates/origin-domain) · [docs.rs](https://docs.rs/origin-domain) |
| `origin-platform` | Platform contracts for Origin: the OS capabilities domain code is allowed to depend on. | [crates.io](https://crates.io/crates/origin-platform) · [docs.rs](https://docs.rs/origin-platform) |

## Platform

| Crate | Purpose | Links |
| --- | --- | --- |
| `origin-events` | Typed, in-process event bus for Origin. No string topics, no untyped payloads. | [crates.io](https://crates.io/crates/origin-events) · [docs.rs](https://docs.rs/origin-events) |
| `origin-secrets` | SecretStore contract for Origin, with an in-memory implementation and a shared contract test suite. | [crates.io](https://crates.io/crates/origin-secrets) · [docs.rs](https://docs.rs/origin-secrets) |
| `origin-settings` | Typed user settings for Origin, backed by any Storage implementation. | [crates.io](https://crates.io/crates/origin-settings) · [docs.rs](https://docs.rs/origin-settings) |
| `origin-storage` | Storage port and TTL cache for Origin. Knows no storage engine. | [crates.io](https://crates.io/crates/origin-storage) · [docs.rs](https://docs.rs/origin-storage) |
| `origin-http` | HTTP port for Origin: request and response types, rate-limit parsing, status mapping. Knows no HTTP library. | [crates.io](https://crates.io/crates/origin-http) · [docs.rs](https://docs.rs/origin-http) |
| `origin-auth` | OAuth 2.0 authorization code flow with PKCE, token storage and refresh for Origin. | [crates.io](https://crates.io/crates/origin-auth) · [docs.rs](https://docs.rs/origin-auth) |
| `origin-accounts` | Account management for Origin: several accounts per connector, credentials kept out of the database. | [crates.io](https://crates.io/crates/origin-accounts) · [docs.rs](https://docs.rs/origin-accounts) |
| `origin-connector` | The connector contract for Origin: what an external service integration must declare and support. | [crates.io](https://crates.io/crates/origin-connector) · [docs.rs](https://docs.rs/origin-connector) |
| `origin-sync` | The Origin sync engine: scheduling, backoff, offline handling and sync state. Knows no external service. | [crates.io](https://crates.io/crates/origin-sync) · [docs.rs](https://docs.rs/origin-sync) |
| `origin-jobs` | Background jobs for Origin: progress, cancellation and a uniform lifecycle. | [crates.io](https://crates.io/crates/origin-jobs) · [docs.rs](https://docs.rs/origin-jobs) |
| `origin-mcp-core` | The MCP boundary: which of an application's operations an external AI may invoke, and under which permission. | [crates.io](https://crates.io/crates/origin-mcp-core) · [docs.rs](https://docs.rs/origin-mcp-core) |
| `origin-ai` | The AI port for Origin: inference the application performs itself, behind a swappable adapter. | [crates.io](https://crates.io/crates/origin-ai) · [docs.rs](https://docs.rs/origin-ai) |
| `origin-telemetry` | Tracing setup and logging conventions for Origin. | [crates.io](https://crates.io/crates/origin-telemetry) · [docs.rs](https://docs.rs/origin-telemetry) |
| `origin-app` | Composition root machinery for Origin applications: ApplicationBuilder, modules, service registry. | [crates.io](https://crates.io/crates/origin-app) · [docs.rs](https://docs.rs/origin-app) |

## Tooling

| Crate | Purpose | Links |
| --- | --- | --- |
| `origin-manifest` | The Origin app manifest: what a product is, parsed and validated. | [crates.io](https://crates.io/crates/origin-manifest) · [docs.rs](https://docs.rs/origin-manifest) |
| `origin-xtask` | Origin maintenance tasks as a library, so a derivative's xtask is three lines and its rules arrive with a version bump. | [crates.io](https://crates.io/crates/origin-xtask) · [docs.rs](https://docs.rs/origin-xtask) |

## Adapters

| Crate | Purpose | Links |
| --- | --- | --- |
| `origin-storage-sqlite` | SQLite-backed Storage adapter for Origin. | [crates.io](https://crates.io/crates/origin-storage-sqlite) · [docs.rs](https://docs.rs/origin-storage-sqlite) |
| `origin-secrets-system` | SecretStore adapter backed by the operating system credential store. | [crates.io](https://crates.io/crates/origin-secrets-system) · [docs.rs](https://docs.rs/origin-secrets-system) |
| `origin-http-reqwest` | HttpClient adapter backed by reqwest. | [crates.io](https://crates.io/crates/origin-http-reqwest) · [docs.rs](https://docs.rs/origin-http-reqwest) |
| `origin-auth-loopback` | Loopback redirect listener for the OAuth authorization code flow (RFC 8252). | [crates.io](https://crates.io/crates/origin-auth-loopback) · [docs.rs](https://docs.rs/origin-auth-loopback) |
| `origin-mcp-stdio` | MCP over stdio: the transport a desktop AI client uses to start an application headless. | [crates.io](https://crates.io/crates/origin-mcp-stdio) · [docs.rs](https://docs.rs/origin-mcp-stdio) |
| `origin-mcp-http` | MCP over a local loopback HTTP endpoint: the transport a desktop AI client uses to reach an already-running GUI instance. | [crates.io](https://crates.io/crates/origin-mcp-http) · [docs.rs](https://docs.rs/origin-mcp-http) |
| `origin-process-std` | ProcessRunner over the local machine: start allowlisted programs and read their output. | [crates.io](https://crates.io/crates/origin-process-std) · [docs.rs](https://docs.rs/origin-process-std) |
| `origin-workspace-fs` | WorkspaceFs over the local filesystem: read-only access under user-confirmed roots. | [crates.io](https://crates.io/crates/origin-workspace-fs) · [docs.rs](https://docs.rs/origin-workspace-fs) |
| `origin-workspace-watch` | WorkspaceWatcher over the notify crate: event-driven local sync for working-tree changes. | [crates.io](https://crates.io/crates/origin-workspace-watch) · [docs.rs](https://docs.rs/origin-workspace-watch) |
| `origin-notifications-tauri` | NotificationService adapter backed by tauri-plugin-notification. | [crates.io](https://crates.io/crates/origin-notifications-tauri) · [docs.rs](https://docs.rs/origin-notifications-tauri) |

## Host

| Crate | Purpose | Links |
| --- | --- | --- |
| `origin-tauri` | Tauri host layer for Origin: plugin wiring, tray, IPC commands and the event bridge. | [crates.io](https://crates.io/crates/origin-tauri) · [docs.rs](https://docs.rs/origin-tauri) |

## Frontend packages

| Package | Purpose | Links |
| --- | --- | --- |
| `@casoon/origin-client` | The only package that speaks Tauri IPC: typed commands, platform events and generated types. | [npm](https://www.npmjs.com/package/@casoon/origin-client) |
| `@casoon/origin-ui` | Shared Svelte 5 components and design tokens. | [npm](https://www.npmjs.com/package/@casoon/origin-ui) |

## Not published

- `origin-demo` (`examples/demo/src-tauri`) — the reference application.
- `xtask` — the repository's own `cargo xtask` entry point; the tasks live in `origin-xtask`.

The publishing order and procedure are in [Publishing](../../lifecycle/publishing/).
