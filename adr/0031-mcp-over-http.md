# ADR-0031  MCP over Local HTTP Loopback

Status:   Accepted
Date:     2026-09-11

## Context

ADR-0027 established MCP as a driving adapter: an external AI client drives application services
via JSON-RPC tools.

The standard stdio transport assumes the client spawns the application process on demand.
However, in a desktop application:
1. The user frequently already has the GUI open. Spawning a second desktop instance either fails
   (due to single-instance locking) or starts duplicate background workers fighting over the database.
2. External AI tools (e.g. Claude Desktop, Cursor, local agent runtimes) need to interact with the
   active desktop session without restarting it.
3. Multiple clients or repeated tool calls should not require process re-spawns.

## Decision

**Origin provides an HTTP loopback transport for MCP (`origin-mcp-http`) bound to `127.0.0.1:0`.**

```text
  CLI / stdio (headless / no GUI running)      HTTP loopback (GUI already running)
  ───────────────────────────────────────      ───────────────────────────────────
  client ──spawn──▶ app (headless) ──stdio     client ──POST──▶ 127.0.0.1:<port>/mcp
                                                                      │
                                                GUI prompt confirms ──┘ (G19 Bearer token)
```

1. **Ephemeral loopback bind (`127.0.0.1:0`)**:
   Like OAuth loopback redirects (ADR-0015), the server binds to port `0` so the OS allocates a free
   ephemeral port. Two running Origin products never collide.
2. **Discovery via runtime file**:
   The bound port, product ID, and endpoint path (`/mcp`) are published to a runtime discovery file
   in the user's application runtime directory (`mcp-http.json`). External clients read this file to
   locate the active server.
3. **Guarded by bearer token**:
   Access is authenticated via a 256-bit CSPRNG bearer token (`Authorization: Bearer <token>`).
   The token is negotiated locally: when an external client requests connection, the user must
   confirm via a native GUI prompt (G19 / `ConfirmationService`). Token comparison is constant-time.
4. **Transparent stdio fallback / proxy**:
   When invoked in stdio mode while a GUI instance is already running, the spawned CLI proxy connects
   to the running GUI's loopback endpoint and streams requests/responses transparently, preventing
   split-brain database states.

## Consequences

- An external AI client can attach to an actively running Origin application without process restarts.
- Single-instance desktop guarantees remain intact.
- The loopback endpoint is not open to arbitrary local processes: bearer authentication and user confirmation
  prevent unauthorized access from other applications on the local machine.
- Headless execution over pure stdio remains fully supported when no desktop instance is active.
