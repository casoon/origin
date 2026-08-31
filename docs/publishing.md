# Publishing Origin's crates

Products scaffolded with `cargo xtask new --released` depend on the `origin-*` crates
by registry version, not by path (see ADR-0026 and `crates/origin-xtask/src/scaffold.rs`).
That only works once those crates actually exist on crates.io. Until then, a scaffold
defaults to `--local`, which points at this checkout instead — a workaround for
pre-publish development, not the target state.

This document is the checklist for the real thing. Publishing itself is a manual,
deliberate act (`scripts/publish-crates.sh --execute`); nothing here runs on its own.

## Why these 21, in this order

Not every crate in the workspace is needed for `--released` to work — only the ones
the template's own dependencies pull in, transitively. `origin-ai`, `origin-mcp`,
`origin-auth-loopback` and `origin-mcp-stdio` are used by `examples/demo` only, so they
are excluded for now; add them to `scripts/publish-crates.sh` when a real product needs
one of them as a registry dependency.

The order matters because `cargo publish` verifies a crate by resolving its
dependencies against the registry, not against local paths: a crate cannot be
published before every `origin-*` crate it depends on already exists on crates.io.

```
origin-domain, origin-manifest
origin-events, origin-platform, origin-secrets, origin-storage, origin-http,
  origin-telemetry, origin-connector
origin-settings, origin-auth, origin-http-reqwest, origin-secrets-system,
  origin-storage-sqlite, origin-notifications-tauri, origin-jobs, origin-sync
origin-accounts
origin-app
origin-tauri
origin-xtask
```

(Crates on the same line don't depend on each other and could in principle publish in
any order relative to one another; the script still does them one at a time, in a fixed
order, so a partial run is easy to reason about.)

## Prerequisites

- A crates.io account with publish rights for the `casoon` namespace, and either
  `cargo login` run locally or `CARGO_REGISTRY_TOKEN` set.
- Crate name availability: run `scripts/publish-crates.sh --check-names` before the
  first real publish (needs network access to crates.io; degrades to "could not check"
  per name rather than failing if that access is unavailable).
- All 21 crates are currently at `0.1.0`; that is fine as a first publish.

## Running it

```bash
scripts/publish-crates.sh                  # metadata check only, publishes nothing
scripts/publish-crates.sh --check-names    # + live crates.io name availability
scripts/publish-crates.sh --execute        # publishes for real, one crate at a time
```

The metadata check verifies every crate has `description`, `license`, `version`,
`readme` (with a matching `README.md` in the crate directory), `keywords` and
`categories` set and valid (checked against the crates.io category slug list embedded
in the script), and is not marked `publish = false` — everything crates.io itself would
otherwise reject one crate at a time, ten minutes apart. None of this needs network
access; `--check-names` is the one step that does.

`--execute` does, for each crate in order:

1. Skip it if `<name>@<version>` is already on crates.io (see below).
2. `cargo publish --dry-run -p <name>` — a real build-and-package pass against the
   crates.io registry, using whichever of its `origin-*` dependencies are already
   published. This is the per-crate dry run; it cannot run meaningfully for every crate
   ahead of time, only immediately before that crate's own turn, because it needs its
   dependencies to already be live.
3. `cargo publish -p <name>` for real.
4. Poll `cargo info <name>@<version>` (the exact version, not just the crate) until the
   crates.io index has caught up — the next crate's dry run and build both depend on
   this one being resolvable there, not merely accepted.

## If a run fails partway through

Crates already published stay published (crates.io has no unpublish for a used
version). Fix the failure and re-run the same `--execute` command: step 1 above skips
anything already published at the current version, so a partial run resumes on its own
rather than needing the `crates` array trimmed by hand.

## After all 21 are published

Switch the default in `crates/origin-xtask/src/lib.rs::new_from_args` from `--local` to
`--released`-by-default (or leave the flag as is and update `docs/updating.md` and the
CI scaffold step in `.github/workflows/ci.yml` to pass `--released`), and drop the
`--local` path-rewriting once no supported workflow needs it.
