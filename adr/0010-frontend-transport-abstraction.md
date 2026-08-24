# ADR-0010  Frontend Transport Abstraction

Status:   Accepted
Date:     2026-08-23

## Context

`invoke("some_command", …)` scattered across views hard-codes the Tauri IPC model
into every component and makes command renames a runtime problem.

## Decision

Only `@origin/client` knows the transport. Views import typed functions:

```ts
import { alerts } from "@origin/client";
await alerts.acknowledge(id);
```

`invoke` outside `frontend/client` is a rule violation (ARCHITECTURE.md #15).

## Consequences

- The transport can change (IPC, WebSocket, in-process mock) without touching views.
- Components are testable in a browser without a Tauri runtime.
- Every command needs a client wrapper. Accepted.
