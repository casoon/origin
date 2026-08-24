# ADR-0006  Connector Architecture

Status:   Accepted
Date:     2026-08-23

## Context

Every product integrates external services with different authentication, pagination,
rate limits and error semantics. Origin must schedule and observe them uniformly
without flattening their domain models into one universal shape.

## Decision

A connector declares *what* it is and *how* to reach it. The platform decides *when*
and *under what conditions* (the sync engine, Phase 3).

- A connector is identified by `ConnectorId` and described by a `ConnectorDescriptor`:
  display name, authentication kind, and the product permissions it needs.
- Authentication goes through `origin-auth`; credentials through `SecretStore`,
  addressed per account (ADR-0016).
- HTTP goes through the `HttpClient` port (ADR-0014). Rate-limit metadata is reported
  to the platform, never handled ad hoc inside a connector.
- `verify` is the one operation every connector must support: prove that an account's
  credentials still work and return the identity behind them.
- Connector-specific data models stay connector-specific. There is no universal
  `Project` or `Metric` mapping layer.

The contract is deliberately small. It grows when Phase 3 has real scheduling needs and
a second connector to validate against — not before (ADR-0009).

## Consequences

- Adding a connector means implementing one small trait plus its own domain types.
- Anything a connector needs from the platform is visible as a trait bound, so a
  connector cannot quietly reach for the filesystem or the network on its own.
- Sync target declaration is intentionally absent until Phase 3.
