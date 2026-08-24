# ADR-0020  Jobs are Process-Local

Status:   Accepted
Date:     2026-08-23

## Context

Background jobs could survive a restart: persist the queue, resume on launch. That
sounds strictly better until you ask what resuming half a PDF render or a partially
written export actually means.

## Decision

Jobs live in memory and end with the process. A job that was running when the
application quit is simply gone; the work that produced it (a sync, an export) is
re-triggered by whatever triggered it the first time.

Consequences of that choice, made explicit:

- The registry keeps the last 50 finished jobs so the UI has history, then drops the
  oldest. Unbounded growth in a long-running desktop app is a leak.
- A job that panics is recorded as `Failed`, not lost. A job the UI shows as running
  forever is worse than one that reports an error.
- Progress events are throttled to roughly one per percent. A job reporting all ten
  thousand of its steps would fill the event channel and push slow subscribers into
  lag — losing them the *finished* event they actually need.
- Cancellation is cooperative: `cancel` records the request and returns. A job decides
  itself when it can stop safely.

## Consequences

- No queue persistence, no resume logic, no half-finished state to reason about.
- Long work that genuinely must survive a restart needs its own durable design, and its
  own ADR. Nothing in the platform pretends to offer it.
