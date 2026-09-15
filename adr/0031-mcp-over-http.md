# ADR-0031  MCP over Local HTTP Loopback

Status:   Accepted
Date:     2026-09-11
Amended:  2026-09-15 — sessions, host/origin checks, and token handling as implemented

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
                                                       Bearer token (G19) · Mcp-Session-Id
```

1. **Ephemeral loopback bind (`127.0.0.1:0`)**:
   Like OAuth loopback redirects (ADR-0015), the server binds to port `0` so the OS allocates a free
   ephemeral port. Two running Origin products never collide.
2. **Discovery via runtime file**:
   The endpoint URL (port and path `/mcp`) and the bearer token are published to a discovery file in
   the application's data directory (`mcp-http.json`). On Unix the file is created with mode `0600`,
   so only the user running the application can read the token. External clients read this file to
   locate the active server.
3. **Guarded by bearer token and host/origin checks**:
   Every request needs `Authorization: Bearer <token>` — a 256-bit CSPRNG token generated per run and
   compared in constant time. `HttpTransport::bind` requires a token; there is no unauthenticated
   mode. Requests whose `Host` is not the loopback endpoint, or that carry a foreign `Origin`, are
   refused with `403` — that keeps web pages and DNS rebinding out. There is no interactive token
   negotiation: whoever can read the discovery file can connect. Mutating tools still need a human
   through `ConfirmationService` (ADR-0029).
4. **Sessions**:
   Following the MCP Streamable HTTP transport, `initialize` opens a session and the response names
   it in `Mcp-Session-Id`; later requests carry that header. Each session has its own lifecycle, so
   several clients and reconnects work side by side. A request without a session (other than `ping`)
   gets `400`, an unknown session `404`. At most 32 sessions are kept; the least recently used goes
   first.
5. **Transparent stdio fallback / proxy**:
   When invoked in stdio mode while a GUI instance is already running, the spawned CLI proxy connects
   to the running GUI's loopback endpoint and streams requests/responses transparently, preventing
   split-brain database states.

## Consequences

- An external AI client can attach to an actively running Origin application without process restarts.
- Single-instance desktop guarantees remain intact.
- The endpoint is as private as the discovery file: other local users cannot read the token, and web
  pages are refused by the host/origin checks. A process running as the same user can read the file
  and connect — the same trust boundary as the application's own data directory.
- Headless execution over pure stdio remains fully supported when no desktop instance is active.
