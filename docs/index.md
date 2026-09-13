---
title: Overview
description: "Origin is a reference architecture and starter system for modular desktop applications built with Rust and Tauri."
order: 0
---

Origin is not a framework that replaces Tauri, and not a monolithic crate every
application must depend on. It is a set of architecture rules, reusable platform crates,
security conventions and build processes, plus a reference application that demonstrates
all of it. The central rule: **domain code does not know Tauri exists.**

## Getting started

| Page | Purpose |
| --- | --- |
| [Installation](getting-started/installation/) | Prerequisites, a checkout, a scaffolded product, or single crates |
| [Quickstart](getting-started/quickstart/) | Run the demo, understand the layout |

## Concepts

| Page | Purpose |
| --- | --- |
| [Architecture overview](concepts/overview/) | Layers, crates, modules and the Tauri integration |
| [Architecture in the code](concepts/architecture/) | How the layers fit together and why |
| [Security](concepts/security/) | Permission model, capabilities, credential handling |
| [The manifest](concepts/manifest/) | app.toml, generated files, security profiles |

## Guides

| Page | Purpose |
| --- | --- |
| [Creating a module](guides/creating-a-module/) | Add a feature area to an application |
| [Authentication and connectors](guides/authentication/) | OAuth, accounts, connectors, rate limits |
| [Sync and jobs](guides/sync-and-jobs/) | Sync engine, policies, backoff, offline, jobs |
| [AI integration](guides/ai-integration/) | MCP as a driving adapter, the AI port, AI permissions |
| [Testing](guides/testing/) | Contract tests, test doubles, testing without a desktop |

## Project lifecycle

| Page | Purpose |
| --- | --- |
| [Updating a project](lifecycle/updating/) | `cargo xtask new`, migrations, keeping derivatives current |
| [Publishing](lifecycle/publishing/) | Publishing the `origin-*` crates and npm packages, in order |

## Reference

| Page | Purpose |
| --- | --- |
| [Crates and packages](reference/crates/) | Every published crate and package, with crates.io, docs.rs and npm links |

The architecture decisions behind all of this are recorded in
[adr/](https://github.com/casoon/origin/tree/main/adr).
