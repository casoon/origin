# ADR-0017  The Sync Engine Owns Scheduling

Status:   Accepted
Date:     2026-08-23

## Context

Every integration needs to decide when to fetch, what to do after a failure, and how to
avoid two overlapping runs. Left to each connector, those decisions get made differently
every time, tested nowhere, and re-debugged per product.

## Decision

The split is:

```text
A SyncSource decides HOW data is fetched.
The engine decides WHEN, and under which conditions.
```

The engine owns scheduling, retry, backoff, jitter, offline handling, validator
storage, health and single-flight. A `SyncSource` fetches and returns a result. It does
not sleep, retry, or ask whether the machine is online.

- Every target is account-scoped (ADR-0016) and its state lives under the account's
  storage prefix (ADR-0019), so disconnecting an account removes it.
- `SyncEngine::run_due(now)` performs one scheduling pass for a given instant; the
  background loop only calls it on a tick. Scheduling is therefore testable by moving a
  fake clock instead of by sleeping.
- Three entry points with distinct meanings: `run_due` (scheduler), `sync_if_due`
  (automatic triggers — window focus, network returning; throttled by `min_interval`),
  and `sync_now` (the user pressed Refresh; always runs).
- Validators are only replaced when the service sent new ones. A response without an
  ETag must not clear the stored one, or every later sync becomes a full refetch.

## Consequences

- A connector is small and has nothing timing-related to get wrong.
- Changing cadence is a policy value, not a code change.
- The engine cannot express a source that needs its own exotic schedule. When one turns
  up, it gets a policy, not an exception.
