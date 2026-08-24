# ADR-0026  Downstream Verification

Status:   Accepted
Date:     2026-08-24

## Context

ARCHITECTURE.md rule 13 says platform changes must build and test the reference
applications. With derivatives in separate repositories (§42), that does not happen by
itself: Origin's CI is green, and the breakage surfaces days later in a product.

## Decision

Origin verifies its consumers as part of its own CI.

**Today**, the only derivative that exists is the template, so Origin's CI scaffolds a
project from it against the current checkout and runs that project's own `validate`,
`generate --check` and `cargo test`. A platform change that breaks a generated project
fails in Origin's pipeline rather than in someone's repository.

This is more than a smoke test. It proves several things that are otherwise assumed:

- The template still compiles against the current platform.
- `cargo xtask` works in a project with a *different layout* — which is the whole point
  of shipping it as a library, and was quietly broken until this check existed.
- A freshly scaffolded project is valid immediately, rather than red on its first CI run.

**Once real derivatives exist**, the same job gains a matrix that checks each one out
and builds it against `main`, nightly and before a release.

## Consequences

- The scaffold path is exercised on every pull request, so it cannot rot unnoticed.
- CI gets slower by one full project build. Worth it.
- A derivative that pins an older Origin version is not covered by this check; that is
  what `cargo xtask update` and its fixtures are for (ADR-0025).
