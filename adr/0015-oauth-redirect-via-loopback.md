# ADR-0015  OAuth Redirect via Loopback

Status:   Accepted
Date:     2026-08-23

## Context

A desktop app finishing an OAuth authorization code flow needs the provider to hand the
code back. Two options exist:

- **Loopback redirect** — the app listens on `http://127.0.0.1:<random port>`.
- **Custom URL scheme** — the app registers `myapp://` with the operating system.

Custom schemes need OS-level registration, behave differently on macOS, Windows and
Linux, are hard to test, break for unpackaged development builds, and can be hijacked by
any other application that registers the same scheme.

## Decision

Loopback redirect, as recommended by RFC 8252 for native applications.

- The listener binds `127.0.0.1:0` — an ephemeral port, so two Origin apps never collide.
- PKCE (S256) is mandatory, including for confidential clients.
- The `state` parameter is generated per flow and verified before the code is used; a
  mismatch aborts the flow.
- The listener accepts exactly one request and then shuts down.
- Redirect handling sits behind a `RedirectListener` port, so a custom-scheme adapter
  can be added later for providers that refuse loopback — without touching the flow.

## Consequences

- Works identically on all three platforms and in unpackaged dev builds.
- Providers that do not allow dynamic loopback ports must be registered with a fixed
  port; the listener supports being pinned to one.
- The flow is testable end to end with a fake listener and a mock HTTP client.
