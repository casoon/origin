# Updating a project

Decisions: [ADR-0025](../adr/0025-project-migrations.md),
[ADR-0026](../adr/0026-downstream-verification.md).

## Two kinds of change

```text
library code        →  cargo / pnpm dependency bump
project structure   →  cargo xtask update
```

The first is solved by package managers. The second is what makes the difference
between a maintainable base and a template that was copied once — and it is what
`update` is for.

## Running it

```bash
cargo xtask update --dry-run   # report what would change
cargo xtask update             # run the migrations, regenerate, record the version
```

Each project records the version it tracks:

```toml
[origin]
version = "0.2.0"
```

`update` runs the migrations between that and the current version, in order, then
regenerates and writes the new version back. Comments and layout in `app.toml` survive
— it is a file a human wrote and will read again.

A project tracking a *newer* version than the running build is refused rather than
downgraded: upgrade the Origin dependency instead.

## What a migration will not do

- It never edits product-owned files (ADR-0022).
- It never guesses where a decision is needed.

Both cases become a checklist:

```text
origin 0.1.0 → 0.2.0
  capability files are generated from app.toml

  Manual steps required:
  → src-tauri/capabilities/default.json grants permissions no profile covers
    (fs:allow-write-text-file, dialog:allow-open). Choose a profile in app.toml, or
    declare `hand_written_capabilities = true` under [origin.overrides].
```

That is the common case for an application that predates Origin. Widening a security
profile to fit is exactly the decision a migration must not make on someone's behalf.

## Deliberate deviations

```toml
[origin.overrides]
hand_written_capabilities = true
```

A migration skips what is listed here and reports it as skipped, rather than
overwriting a decision someone made on purpose (§46).

## How migrations are tested

Against frozen fixture projects in `crates/origin-xtask/tests/fixtures/`, copied into a
scratch directory and migrated. The tests cover the parts that are easy to get wrong:

- a convertible capability is replaced by a generated one
- one that no profile covers is left untouched, with the question handed back
- running twice changes nothing the second time
- `--dry-run` writes nothing
- comments in `app.toml` survive
- a project from the future is refused

Without fixtures, migrations are code that runs exactly once — in someone else's
repository.

## Starting a new project

```bash
cargo xtask new my-app --name "My App" --id dev.example.myapp
```

The result is not a blank window: it is the architecture contract, logging, error
handling, settings, secrets, storage, a security profile, CI, and a module that is
already testable without a desktop session. Its `xtask` is three lines, so the rules
and the generator arrive with a version bump.

`--local` points the dependencies at an Origin checkout instead of at released
versions. That is how Origin's own CI proves the template still builds against `main`
(ADR-0026) — and how you develop the platform and a product side by side.
