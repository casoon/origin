# ADR-0018  Backoff, Jitter and Offline Handling

Status:   Accepted
Date:     2026-08-23

## Context

Fixed-interval retry against a failing service turns an outage into a self-inflicted
load test, and several targets that failed together retry in lockstep.

Being offline is a different failure from a service being broken, but both surface as
"the request did not work".

## Decision

**Exponential backoff with a cap and symmetric jitter.** `Backoff::delay_for(failures,
random)` is a pure function that takes the random value as an argument — jitter is
otherwise untestable. Default: 30 s base, doubling, capped at 30 min, ±20 % jitter.

**Offline gets a flat, short retry** (default 20 s) instead of the exponential curve.
Connectivity usually returns in one step; backing off to half an hour would leave the
application stale long after the network came back. The engine recognises it from
`AppError::Offline`, which adapters already produce for connection failures.

**No `NetworkStatus` port.** A separate connectivity API would report a usable link on
a captive-network Wi-Fi and an unusable one on a VPN that is about to connect. The
attempt itself is the only reliable probe, so offline is a fact derived from failures,
not a state polled in advance.

**`min_interval` throttles triggers, not the scheduler.** It exists so that a UI
re-syncing on every window focus does not mean twenty syncs; applying it to the
scheduler would silently override a configured `offline_retry`.

## Consequences

- A recovering service is not hammered, and targets that failed together spread out.
- One misconfigured policy cannot produce a tight retry loop: the cap holds.
- The engine has no live "are we online" signal. It finds out by trying, which costs
  one failed request per outage window.
