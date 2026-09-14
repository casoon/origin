---
title: Installation
description: "Prerequisites, then three ways in — run Origin from a checkout, scaffold a product from its template, or use single crates on their own."
order: 1
---

## Prerequisites

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

## Run Origin from a checkout

```bash
git clone https://github.com/casoon/origin
cd origin
pnpm install
cargo xtask demo
```

This builds and starts the reference application. The [Quickstart](../quickstart/) explains
what it demonstrates and which files to read first.

## Scaffold a product

From inside the checkout, `cargo xtask new` instantiates the project template in
`templates/app`:

```bash
cargo xtask new my-app --name "My App" --id dev.example.myapp --into ~/projects
```

| Flag | Meaning |
| --- | --- |
| `<slug>` | directory and crate name of the new project |
| `--name` | display name; defaults to the slug in title case |
| `--id` | reverse-DNS product id; defaults to `dev.local.<slug>` |
| `--into` | parent directory for the new project |
| `--local` | depend on this checkout instead of the released crates |

Without `--local`, the generated project depends on the `origin-*` crates and the
`@casoon/origin-*` npm packages by registry version. Its own `xtask` is three lines, so
the architecture rules and the generator arrive with a version bump; see
[Updating a project](../../lifecycle/updating/).

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
