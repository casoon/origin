---
title: Installation
description: "Prerequisites, then three ways in — scaffold a product with the origin CLI, run Origin from a checkout, or use single crates on their own."
order: 1
---

## Prerequisites

Scaffolding needs Rust alone. The rest is what *building* the generated
application needs — a Tauri app is a desktop build, not a web project.

| Tool | Version |
| --- | --- |
| Rust | 1.88 or newer |
| Node | 22 or newer |
| pnpm | 10 or newer |
| Tauri CLI | 2.x (`cargo install tauri-cli --version "^2"`) |

On Linux you also need the Tauri system dependencies:

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev
```

## Scaffold a product

```bash
cargo install origin-cli
origin new my-app --name "My App" --id dev.example.myapp --into ~/projects
```

That is the whole entry path: the project template is compiled into the `origin`
binary, so there is nothing to clone and nothing to keep in sync by hand.

| Flag | Meaning |
| --- | --- |
| `<slug>` | directory and crate name of the new project |
| `--name` | display name; defaults to the slug in title case |
| `--id` | reverse-DNS product id; defaults to `dev.local.<slug>` |
| `--into` | parent directory for the new project; defaults to the current one |
| `--local` | depend on the surrounding Origin checkout instead of the released crates |

The generated project depends on the `origin-*` crates and the `@casoon/origin-*` npm
packages by registry version. Its own `xtask` is three lines, so the architecture rules
and the generator arrive with a version bump; see
[Updating a project](../../lifecycle/updating/).

`--local` is for developing the platform and a product side by side, and for Origin's
own CI; it only works from inside a checkout of Origin and fails with a clear message
elsewhere.

## Run Origin from a checkout

```bash
git clone https://github.com/casoon/origin
cd origin
pnpm install
cargo xtask demo
```

This builds and starts the reference application. The [Quickstart](../quickstart/) explains
what it demonstrates and which files to read first.

From a checkout, scaffolding runs out of the workspace instead of an installed binary —
the template lives in `crates/origin-cli/templates`:

```bash
cargo run -p origin-cli -- new my-app --into ~/projects --local
```

## Use single crates

Every workspace crate except the demo and the repository's own `xtask` is published on
crates.io. They are meant to stay usable outside Origin — in CLIs, services and other
projects:

```bash
cargo add origin-domain origin-events origin-storage
```

The frontend packages are published under the `@casoon` scope:

```bash
pnpm add @casoon/origin-client @casoon/origin-ui
```

The [crate reference](../../reference/crates/) lists every package with its role and
API documentation.

## Checks

```bash
cargo xtask validate    # enforce the architecture rules
cargo xtask ci          # fmt + clippy + test + generated files + validate
cargo test --workspace  # Rust tests, no desktop session required
pnpm -r check           # TypeScript and Svelte checks
```

The system-keychain contract test is excluded by default because it touches your real
login keychain. Run it deliberately:

```bash
cargo test -p origin-secrets-system -- --ignored
```
