# ADR-0022  Generated Files are Origin-Owned

Status:   Accepted
Date:     2026-08-23

## Context

An Origin upgrade can only be automatic where file ownership is unambiguous. A file
that both Origin and the product edit cannot be updated without a merge, and merges are
what turn "upgrade the platform" into a day of work per project.

## Decision

Every file in a derivative has exactly one of three roles:

| Role | Update behaviour |
| --- | --- |
| **origin-owned** | regenerated wholesale; never edited by hand |
| **shared** | Origin provides a baseline, the product extends it through `app.toml` |
| **product-owned** | never touched by Origin |

**Generated means origin-owned.** If `cargo xtask generate` produces a file, that file
is Origin's. It carries a marker saying so, and `cargo xtask generate --check` runs in
CI and fails on a hand-edit or a stale file.

Two consequences of the mechanism are deliberate:

- The generator only removes files it recognises as its own, by marker. A hand-written
  capability keeps working until someone converts it; a generator that deletes
  unfamiliar files is a generator nobody trusts.
- Generation writes only when content changed, so an unchanged run does not touch
  timestamps and does not trigger a rebuild.

Changing an origin-owned file means changing `app.toml` — or, for what a profile
*means*, changing it in `origin-manifest`, where it changes for every Origin
application at once and is reviewed once instead of per project.

## Consequences

- The set of files an upgrade may overwrite is knowable, mechanically.
- A hand-edit of generated config is caught in CI rather than silently surviving until
  the next regeneration wipes it.
- Anything not generated still needs a migration when it changes. Keeping that set
  small is the ongoing job (see the update system plan, category B).
