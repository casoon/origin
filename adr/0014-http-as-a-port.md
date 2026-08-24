# ADR-0014  HTTP as a Port

Status:   Accepted
Date:     2026-08-23

## Context

Every connector talks HTTP. If each one constructs its own `reqwest::Client`, the
application ends up with several connection pools, several timeout policies, no shared
place to observe requests, and connector tests that need a network or a local server.

## Decision

HTTP is a port: `HttpClient` in `origin-http`, implemented by
`adapters/origin-http-reqwest`.

- `reqwest` is the default implementation. It stays an implementation detail; no
  `reqwest` type appears in a connector signature.
- SDK crates (`github-api`, …) take an `Arc<dyn HttpClient>`. They remain context-free
  (ADR-0003) and testable without a network.
- Rate-limit metadata is parsed once, in `origin-http`, from the standard `RateLimit-*`
  and `Retry-After` headers plus the widespread `X-RateLimit-*` variants. Connectors
  report it; they do not each re-implement it.
- HTTP status codes are mapped to `AppError` in one place, so `401`, `429` and `5xx`
  mean the same thing to the UI regardless of which service produced them.

## Consequences

- One connection pool, one timeout policy, one place to add tracing.
- `MockHttpClient` makes connector tests deterministic and offline.
- A second implementation (recording proxy, offline replay) is possible without
  touching connectors.
- One indirection between a connector and the wire. Accepted.
