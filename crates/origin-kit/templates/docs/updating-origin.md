# Updating Origin

Origin is a platform you track, not a template you copied once. Two kinds of change
reach this project by two different routes:

```text
library code        →  cargo / pnpm dependency bump
project structure   →  cargo xtask update
```

## Library code

Bump the versions in `Cargo.toml` and `ui/package.json`. Normal dependency work; a
breaking change comes with a migration note in Origin's release.

## Project structure

Configuration, generated files, CI recipes and the architecture rules cannot travel
through Cargo. That is what `update` is for:

```bash
cargo xtask update --dry-run   # what would change
cargo xtask update             # do it
```

This project records the version it tracks:

```toml
[origin]
version = "__ORIGIN_VERSION__"
```

`update` runs the migrations between that and the version your Origin dependency knows,
regenerates, and writes the new version back. Comments and layout in `app.toml` survive
— it is a file you wrote and will read again.

It produces a normal diff in this repository. Review it like any other change.

## What it will not do

A migration never edits your code, and never guesses where a decision belongs to you.
Both come back as a checklist:

```text
origin 0.1.0 → 0.2.0
  capability files are generated from app.toml

  Manual steps required:
  → src-tauri/capabilities/default.json grants permissions no profile covers
    (fs:allow-write-text-file). Choose a profile in app.toml, or declare
    `hand_written_capabilities = true` under [origin.overrides].
```

Widening a security profile to fit is exactly the decision an automated step must not
make on your behalf.

## Deliberate deviations

```toml
[origin.overrides]
hand_written_capabilities = true
```

An update skips what is listed here and reports it as skipped. Write down *why*, in a
comment or in your own decision record — the next person to read it will be you.

## The rules travel too

`xtask/src/main.rs` is three lines. The tasks — architecture validation, the generator,
the migrations, the CI recipe — live in `origin-xtask`. So a new architecture rule
arrives with a version bump and fails CI the same day, instead of sitting in a document
nobody re-reads.

That is the trade: this project does not own those rules, and in exchange it never has
to chase them.
