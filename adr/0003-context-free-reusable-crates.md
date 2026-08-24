# ADR-0003  Context-Free Reusable Crates

Status:   Accepted
Date:     2026-08-23

## Context

Anything reusable beyond a single application tends to accumulate assumptions about
the app it was born in — storage, logging, notification behaviour, product naming.

## Decision

Reusable functionality is developed as an independent crate that knows nothing about
Origin, the product, Tauri, the UI framework, or the storage engine.

- `github-api` knows GitHub. It does not know Gitbit, Origin, SQLite or Tauri.
- `origin-github` may know the Origin domain model. `github-api` may not.
- Prefer existing high-quality crates over writing new ones.

## Consequences

- SDK crates stay publishable and usable outside Origin.
- One extra integration crate per external service.
- Product names in a `crates/` dependency graph are a rule violation (ARCHITECTURE.md #2).
