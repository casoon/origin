# ADR-0005  Events versus Direct Calls

Status:   Accepted
Date:     2026-08-23

## Context

Event buses make systems flexible and untraceable in equal measure. Used for
everything, control flow disappears into the bus.

## Decision

- Need a result now? Call the service directly (`get_project`, `store_secret`).
- Several independent components may react? Publish a **typed** event.
- Events are Rust types on a typed bus. No string topics, no `serde_json::Value` payloads.

## Consequences

- Adding a subscriber never touches the publisher.
- A renamed event field is a compile error, not a silent runtime mismatch.
- Contributors must make a conscious choice per interaction. Intended.
