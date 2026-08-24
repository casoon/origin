# ADR-0011  Manifest and Code Generation

Status:   Superseded by ADR-0021
Date:     2026-08-23

## Context

Without a single declarative source, applications drift apart in Tauri config,
capabilities, plugin registration and default settings.

## Decision (draft, Phase 4)

Each product declares itself in `app.toml`; `cargo xtask generate` derives Tauri
config, capability definitions, plugin registration, feature flags and frontend
configuration from it. Generated files are checked in and CI verifies they are current.

## Consequences

- Open until Phase 4. Until then configuration is hand-written but kept minimal.
- Generated files must never be edited by hand.
