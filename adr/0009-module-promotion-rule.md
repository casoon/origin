# ADR-0009  Module Promotion Rule

Status:   Accepted
Date:     2026-08-23

## Context

The first heavy consumer of a platform silently turns it into its own framework.
Gitbit will be that consumer.

## Decision

**Gitbit may drive Origin, but never bypass it.**

A feature starts where it is needed. It is promoted into `crates/` only when a
genuinely neutral abstraction exists — in practice at the **third** occurrence
(Rule of Three), not the first similarity.

```text
Repository Groups (Gitbit) → Website Groups (Metricbit) → Zone Groups (Cloudbit)
                                                            ↓
                                                    ResourceCollection
```

## Consequences

- Some duplication is tolerated on purpose, for a while.
- Removing a premature abstraction is an accepted, normal change.
