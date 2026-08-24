# Architecture Decision Records

One decision per file, short and readable. Format:

```text
# ADR-XXXX  Title

Status:   Proposed | Accepted | Superseded by ADR-YYYY
Date:     YYYY-MM-DD

## Context
## Decision
## Consequences
```

An ADR is never edited to change its decision — it is superseded by a new one.

| ADR | Title | Status |
| --- | --- | --- |
| [0001](0001-tauri-as-host-adapter.md) | Tauri as Host Adapter | Accepted |
| [0002](0002-rust-first-domain-architecture.md) | Rust-first Domain Architecture | Accepted |
| [0003](0003-context-free-reusable-crates.md) | Context-Free Reusable Crates | Accepted |
| [0004](0004-dependency-injection-strategy.md) | Dependency Injection Strategy | Accepted |
| [0005](0005-events-versus-direct-calls.md) | Events versus Direct Calls | Accepted |
| [0006](0006-connector-architecture.md) | Connector Architecture | Accepted |
| [0007](0007-security-and-capability-model.md) | Security and Capability Model | Accepted |
| [0008](0008-local-cache-and-source-of-truth.md) | Local Cache and Source of Truth | Accepted |
| [0009](0009-module-promotion-rule.md) | Module Promotion Rule | Accepted |
| [0010](0010-frontend-transport-abstraction.md) | Frontend Transport Abstraction | Accepted |
| [0011](0011-manifest-and-code-generation.md) | Manifest and Code Generation | Superseded by ADR-0021 |
| [0012](0012-distribution-architecture.md) | Distribution Architecture | Superseded by ADR-0030 |
| [0013](0013-frontend-stack.md) | Frontend Stack: Svelte 5 + Vite + Tailwind v4 | Accepted |
| [0014](0014-http-as-a-port.md) | HTTP as a Port | Accepted |
| [0015](0015-oauth-redirect-via-loopback.md) | OAuth Redirect via Loopback | Accepted |
| [0016](0016-multi-account-from-day-one.md) | Multi-Account from Day One | Accepted |
| [0017](0017-sync-engine-owns-scheduling.md) | The Sync Engine Owns Scheduling | Accepted |
| [0018](0018-backoff-jitter-and-offline.md) | Backoff, Jitter and Offline Handling | Accepted |
| [0019](0019-storage-namespace-convention.md) | Storage Namespace Convention | Accepted |
| [0020](0020-jobs-are-process-local.md) | Jobs are Process-Local | Accepted |
| [0021](0021-manifest-is-the-single-source.md) | The Manifest is the Single Source | Accepted |
| [0022](0022-generated-files-are-origin-owned.md) | Generated Files are Origin-Owned | Accepted |
| [0023](0023-design-tokens-are-a-contract.md) | Design Tokens are a Stable Contract | Accepted |
| [0024](0024-typescript-contracts-are-generated.md) | TypeScript Contracts are Generated from Rust | Accepted |
| [0025](0025-project-migrations.md) | Origin Version and Project Migrations | Accepted |
| [0026](0026-downstream-verification.md) | Downstream Verification | Accepted |
| [0027](0027-mcp-is-a-driving-adapter.md) | MCP is a Driving Adapter, not an Inference API | Accepted |
| [0028](0028-the-ai-provider-port.md) | The AI Provider Port | Accepted |
| [0029](0029-the-ai-permission-level.md) | The AI Permission Level | Accepted |
| [0030](0030-distribution.md) | Distribution | Accepted |
