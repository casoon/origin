# ADR-0024  TypeScript Contracts are Generated from Rust

Status:   Accepted
Date:     2026-08-23

## Context

Every type that crosses the IPC boundary existed twice: once in Rust and once,
hand-written, in `@origin/client`. The mirror is correct on the day it is written and
drifts from then on. The failure mode is the bad kind — no build error, just
`undefined` at runtime, in production, on a field somebody renamed weeks ago.

The file even carried a comment admitting it: *"change a type here whenever you change
it in Rust"*. Instructions in a comment are not a mechanism.

## Decision

TypeScript bindings are generated from the Rust definitions with `ts-rs`, into
`frontend/client/src/generated.ts`. `cargo xtask generate --check` runs in CI, so a
Rust change without a regenerated file is a red build (ADR-0022).

Consequences of that choice, made deliberately:

- **Contract types live with their domain, not with the transport.** Moving `AppInfo`
  and `SyncStatus` out of the host layer was part of this: `origin-xtask` must not
  depend on Tauri, and a contract type in the host layer cannot be exported.
- **`ts` is an optional feature.** Shipped builds do not compile `ts-rs`.
- **Large integers are emitted as `number`, not `bigint`.** `serde_json` writes `u64`
  as a JSON number and the IPC layer hands JavaScript a `number`; declaring `bigint`
  would describe a value that never arrives. Anything above 2^53 loses precision on
  this path regardless — a contract needing such values must carry them as strings.
- **Timestamps are declared as `string`.** They are serialised RFC 3339, and the Rust
  type says nothing about that.

## Consequences

- Adding an event variant in Rust breaks the frontend's exhaustive `switch` at check
  time. That happened on the first run of this generator, on the job events added in
  Phase 3 — exactly the drift the mirror had been hiding.
- Field names in TypeScript follow serde, so they stay `snake_case`. Renaming them for
  frontend taste would reintroduce a mapping layer, which is the problem this removes.
- The export list in `origin-xtask` has to be maintained. A type that is not listed is
  simply not available to the frontend, which is a visible absence rather than a silent
  mismatch.
