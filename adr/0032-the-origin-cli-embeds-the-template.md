# ADR-0032  The `origin` CLI Embeds the Template

Status:   Accepted
Date:     2026-11-09

## Context

Scaffolding was `cargo xtask new`, which ran out of `origin-xtask` and read the template
from `templates/app` in the *repository*. Creating a project therefore meant cloning an
18-crate workspace and building it, and the error message said so ("scaffolding
currently needs the Origin repository"). The template itself is small — 45 files and
151 lines of Rust — so the entry cost was out of all proportion to what it delivered.

`origin-xtask` is also a dependency of every generated project, so scaffolding code and
the template's bytes would have shipped inside every derivative's build tooling had the
template simply been embedded there.

## Decision

Scaffolding moves into its own crate, **`origin-cli`**, which installs the `origin`
binary. The template is compiled into that binary (`build.rs` generates an
`include_bytes!` table), so `origin new` needs Rust and nothing else.

- **`origin-xtask` keeps what a project runs on itself** — `validate`, `generate`,
  `update`, `ci` — and exports the Origin version it tracks (`CURRENT_VERSION`), so the
  tool that creates a project and the migrations that later move it cannot disagree
  about where it started. `cargo xtask new` is removed.
- **The template lives in `crates/origin-cli/templates`.** `cargo package` skips every
  directory that holds a `Cargo.toml`, which would leave the published crate without a
  template. The template's three manifests are therefore stored as `Cargo.toml.in` and
  written under their real names. A test fails if a template manifest is ever named
  `Cargo.toml`, because the failure is invisible in a checkout and appears only after
  `cargo install origin-cli`.
- **`--local` stays**, and fails with a message outside a checkout of Origin instead of
  guessing a root.
- **No argument parser.** One command with four flags does not need one.

## Consequences

- `cargo install origin-cli && origin new my-app` is the entry path; no clone.
- Origin's own CI scaffolds through `cargo run -p origin-cli -- new … --local`; ADR-0026
  is unchanged in substance.
- One more published crate (30), published after `origin-xtask`.
- The embedded template is the version of the CLI that was installed. A project created
  with an old binary starts at that Origin version and is brought forward by
  `cargo xtask update` (ADR-0025), as before.
- Scaffold *profiles* (`--profile minimal|connected|full`) become possible without a
  second template, because the template is now data the CLI owns (plan 08, part C).
