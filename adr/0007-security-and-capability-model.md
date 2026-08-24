# ADR-0007  Security and Capability Model

Status:   Accepted
Date:     2026-08-23

## Context

The default Tauri path — grant the frontend broad `fs` and `shell` permissions —
turns any frontend XSS into arbitrary local code execution.

## Decision

Two separate permission levels, never mixed:

- **Product permissions** — external rights (`ReadNotifications`, `WriteProjects`, …)
- **Platform permissions** — local rights (Filesystem, Shell, Process, Credential Store, …)

Tauri capabilities are scoped per window via named security profiles
(`readonly-dashboard`, `standard-dashboard`, `account-settings`, `local-workspace`).
Blanket `fs:*` / `shell:*` is prohibited. Tokens never go into plain SQLite tables;
they go through `SecretStore` (ADR-0008).

## Consequences

- Each new window requires an explicit profile decision.
- Adding a permission is visible in review as a capability diff.
