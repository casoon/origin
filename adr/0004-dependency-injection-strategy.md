# ADR-0004  Dependency Injection Strategy

Status:   Accepted
Date:     2026-08-23

## Context

Ports-and-adapters needs wiring. Enterprise-style DI containers with runtime
resolution, string keys and reflection are a poor fit for Rust and hide the graph.

## Decision

Wiring is explicit and compile-time checked:

- ports are traits, held as `Arc<dyn Trait>`,
- `ApplicationBuilder` assembles them,
- every product has exactly one **composition root**,
- sensible defaults via `with_default_*`, overridable per component.

Principle: **convention by default, explicit override when needed.**

## Consequences

- The composition root doubles as product architecture documentation.
- No runtime resolution failures; a missing dependency is a build error.
- Builders grow with the platform. Accepted — they stay greppable.
