# ADR-0021  The Manifest is the Single Source

Status:   Accepted
Date:     2026-08-23
Supersedes the draft in ADR-0011.

## Context

Configuration that is written by hand in each project drifts. Two Origin applications
end up with different Tauri settings, different capability layouts and different CI
recipes, and a platform improvement has to be applied N times by hand.

That cost is what decides whether Origin is a maintainable base or a template that was
copied once.

## Decision

`app.toml` declares **what a product is**. The composition root declares **how it is
assembled**. Anything derivable from the first is generated, never hand-written:

```text
app.toml ──generate──▶ capabilities/*.json
                       (later: tauri config, plugin registration, frontend config)
```

The format stays deliberately small. Every field in it becomes migration-liable the
moment a second product exists, so a field is added only when something is actually
derived from it.

Validation happens at parse time and covers what types cannot express — a
non-reverse-DNS product id, a window without a security profile, or a tray application
that permits second instances (which would mean a second tray icon).

`[origin.overrides]` records deliberate deviations (§46). A migration skips what is
listed there and reports it as a manual step instead of overwriting a decision someone
made on purpose.

## Consequences

- Configuration drift between products becomes a CI failure rather than a discovery.
- Security profiles are chosen from named options rather than assembled per project.
- Adding a manifest field is a commitment. Deriving nothing from a field is a reason
  not to add it.
