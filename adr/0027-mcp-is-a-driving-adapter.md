# ADR-0027  MCP is a Driving Adapter, not an Inference API

Status:   Accepted
Date:     2026-08-24

## Context

Every application is growing an AI feature, and each one arrives with its own API key
field, its own subscription and its own chat window:

```text
Tool A → its own subscription      Tool D → an "AI Pro" upgrade
Tool B → your OpenAI key           Tool E → its own usage again
Tool C → your Anthropic key
```

What a user actually wants is to point the AI they already pay for at the applications
they already use.

The tempting reading of MCP is that it solves this by letting an application borrow the
user's model. It does not: MCP connects an AI client to a server's tools. The data flows
the other way round.

## Decision

**MCP makes the application controllable by an external AI. It is never used as an
inference API.**

```text
                          ┌── MCP server   someone else's AI drives the app
UI ── Application Core ───┤
                          └── AiService    the app performs inference itself
```

The two are independent, separately switchable, and neither implies the other.

An MCP server is a **driving adapter** — the same architectural role as the Tauri host
or a CLI. It follows that:

- Nothing in `origin-mcp` knows Tauri. The demo serves MCP over stdio with no window,
  no desktop session and no display.
- A tool wraps an *application service*, never a command. An operation that exists only
  as a Tauri command is not reachable from MCP, a CLI or a headless run — and writing a
  tool for it would mean duplicating logic, which is the signal that the logic sits in
  the wrong place.
- The application needs no model, no key and no chat window of its own.

The wire protocol is JSON-RPC 2.0 with `initialize`, `tools/list` and `tools/call`.
**The exact revision must be verified against the current specification before
connecting a real client** — the boundary in this crate is the durable part; the
envelope is replaceable.

## Consequences

- A user brings their own AI *client*, not their own API key.
- The headless path is exercised by tests, so the claim that the core runs without Tauri
  is checked rather than asserted.
- Tool descriptions become public API for a reader that cannot ask follow-up questions.
  They belong in review like any other interface.
