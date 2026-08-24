# ADR-0025  Origin Version and Project Migrations

Status:   Accepted
Date:     2026-08-24

## Context

Library code updates through Cargo and pnpm. Project structure and configuration
cannot: a new capability layout, a changed CI recipe, a renamed config field. Left
manual, each of those costs one afternoon per derivative, and the fourth project never
gets updated at all.

That cost is what decides whether Origin stays a maintainable base or becomes a
template that was copied once.

## Decision

Every project records the Origin version it tracks:

```toml
[origin]
version = "0.2.0"
```

`cargo xtask update` runs the migrations between that version and the current one, in
order, then regenerates and writes the new version back.

Properties each migration must have, and why:

- **Idempotent.** Running twice must change nothing the second time. Someone will run
  it twice.
- **Reviewable.** It produces a normal diff in the project's own repository, not a
  black-box result.
- **Format-preserving.** `app.toml` is a file a human wrote and will read again, so
  comments and layout survive (`toml_edit`, not re-serialisation).
- **Willing to stop.** A migration never edits product-owned files (ADR-0022), and
  never guesses where a decision is needed. It reports a checklist instead.
- **Tested against frozen fixtures.** Otherwise migrations are code that runs exactly
  once, in someone else's repository.

The first migration is the one the first real derivative needs: converting a
hand-written Tauri capability file into a security profile. Where the file grants
permissions no profile covers — the common case for an application that predates
Origin — it is left untouched and the question is handed back. Widening a profile to
fit is exactly the decision a migration must not make on someone's behalf.

A project tracking a *newer* version than the running build is refused rather than
downgraded.

## Consequences

- A platform improvement reaches four derivatives as four command runs.
- Every structural change now needs a migration, or a documented reason it needs none.
- Migrations accumulate. They are cheap to keep and expensive to reconstruct, so they
  stay.
