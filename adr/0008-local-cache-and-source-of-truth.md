# ADR-0008  Local Cache and Source of Truth

Status:   Accepted
Date:     2026-08-23

## Context

Once a local database holds the same data as the remote service, ownership becomes
ambiguous and conflict resolution creeps in.

## Decision

External services remain **Source of Truth** unless explicitly documented otherwise.

SQLite is used for: cache, read models, local state, sync metadata, user settings.
It is **not** used for credentials — those go to the system keychain via `SecretStore`.

Deleting the local database must never lose user data.

## Consequences

- Any locally-owned data needs its own ADR and a backup story.
- Cache invalidation is a platform concern (ETag / Last-Modified / TTL), not per feature.
