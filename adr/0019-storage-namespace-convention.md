# ADR-0019  Storage Namespace Convention

Status:   Accepted
Date:     2026-08-23

## Context

Every module writes to storage. Without a convention, each invents its own key layout,
and two questions become unanswerable: *what belongs to this account*, and *what may be
deleted when the user disconnects it?*

This is expensive to change later — it touches every module of every derivative — so it
is decided before a second application exists.

## Decision

Three namespace shapes, no exceptions:

```text
origin.<area>                      platform data, not tied to an account
                                   origin.settings · origin.accounts

acct.<connector>.<account>.<area>  anything belonging to one connected account
                                   acct.github.a1b2.notifications · …sync

app.<module>.<area>                product data with no account behind it
                                   app.planning.templates
```

`Storage` gains `clear_prefix(prefix)`. Disconnecting an account clears
`acct.<connector>.<account>.` and is therefore complete by construction: no module has
to register which namespaces it wrote, and no module can forget to.

The trailing separator is load-bearing — without it, disconnecting account `a1` would
also delete `a1b2`. The contract test suite covers exactly that case.

## Consequences

- Removing an account is one call, not a cleanup protocol between modules.
- Modules must use the helpers in `origin_storage::namespace` rather than formatting
  keys by hand. `cargo xtask validate` cannot check this yet — a candidate rule.
- Account-scoped data cannot be shared between two accounts. That is intended.
