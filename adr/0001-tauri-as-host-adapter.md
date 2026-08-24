# ADR-0001  Tauri as Host Adapter

Status:   Accepted
Date:     2026-08-23

## Context

Tauri provides windows, tray, native notifications, autostart, updater, shell and
filesystem access. It is tempting to build the application *inside* Tauri and pass
`AppHandle` around. That couples every service to a specific Tauri major version and
makes the domain untestable without a running desktop session.

## Decision

Tauri is a **host adapter**, not the architecture.

- No crate under `crates/` depends on `tauri` or `tauri-plugin-*`.
- OS capabilities are expressed as traits in `crates/origin-platform`.
- Tauri-backed implementations live in `adapters/*-tauri` and `host/origin-tauri`.
- A domain service never receives an `AppHandle`.

## Consequences

- A Tauri 2 → 3 migration is limited to `host/` and `adapters/*-tauri`.
- The same core can drive a CLI or a headless agent (see ADR-0002).
- One extra indirection per OS capability. Accepted.
