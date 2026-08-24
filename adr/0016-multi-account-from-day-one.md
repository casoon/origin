# ADR-0016  Multi-Account from Day One

Status:   Accepted
Date:     2026-08-23

## Context

Single-account is the tempting simplification: one token, one identity, no selector in
the UI. But Cloudbit needs several Cloudflare accounts, Metricbit several GA4
properties across clients, and Gitbit work and personal GitHub accounts. Retrofitting
multi-account means changing every cache key, every sync record and every UI surface.

## Decision

An account is a first-class entity from the start.

- `AccountId` is required, not optional, on anything that reaches an external service:
  cache keys, sync state, alerts, credentials.
- A connector may have zero, one or many accounts.
- Credentials are stored per account, addressed by `(connector, account)`, so revoking
  one account never touches another.
- The account list itself lives in `Storage`; only the tokens live in `SecretStore`.

## Consequences

- Slightly more plumbing in single-account products. Accepted — it is far cheaper than
  the migration would be.
- Cache and sync data are naturally partitioned per account, so removing an account is a
  namespace delete.
- The UI needs an account selector once a product has more than one. Until then it can
  simply render the only account.
