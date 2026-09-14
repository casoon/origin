---
title: AI integration
description: "MCP as a driving adapter for the user's own AI client, the AI permission level, and inference the application performs itself."
order: 4
---

Decisions: [ADR-0027](https://github.com/casoon/origin/blob/main/adr/0027-mcp-is-a-driving-adapter.md),
[ADR-0028](https://github.com/casoon/origin/blob/main/adr/0028-the-ai-provider-port.md),
[ADR-0029](https://github.com/casoon/origin/blob/main/adr/0029-the-ai-permission-level.md),
[ADR-0031](https://github.com/casoon/origin/blob/main/adr/0031-mcp-over-http.md).

## Two boundaries, not one

```text
                          ┌── MCP server   someone else's AI drives the app
UI ── Application Core ───┤
                          └── AiService    the app performs inference itself
```

They are independent and separately switchable. Conflating them is the common mistake:
MCP connects an AI *client* to a server's tools — it does not let an application borrow
the user's model subscription.

Neither is required. The application is fully usable with both switched off. **AI is
never the foundation.**

## Bring your own AI client

Not *bring your own API key*. The user points the AI they already pay for at the
application:

```text
                   the user's AI
             ┌──────────┼──────────┐
            MCP        MCP        MCP
             │          │          │
          Tool A     Tool B     Tool C
```

The application needs no model, no key and no chat window of its own.

## Try it

The demo serves MCP on stdio:

```bash
cargo build -p origin-demo
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' \
  | ./target/debug/origin-demo --mcp
```

No window opens. That is the architecture test from §52/§53 of the concept, executed:
the same core that drives the desktop shell answers an AI client, and neither knows
about the other.

## Writing a tool

A tool wraps an **application service**, never a Tauri command:

```rust
#[async_trait]
impl Tool for StatusTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor::new(
            "demo.status",
            "Read status",
            "Returns the current load reading, the derived health state and any active \
             alerts. Use this before proposing a threshold change.",
            AiPermission::Read,
        )
    }

    async fn call(&self, _arguments: Value) -> Result<ToolOutput> {
        let snapshot = self.pulse.snapshot().await?;
        Ok(ToolOutput::text(/* … */))
    }
}
```

If writing a tool would mean duplicating logic from a command, the logic is in the wrong
place. An operation that exists only as a command is unreachable from MCP, from a CLI
and from a headless run.

The **description is public API** for a reader that cannot ask follow-up questions. It
belongs in review like any other interface.

Arguments come from a model. They are input, not a contract — validate them.

## Permissions

The caller is a model acting on content it read somewhere, and that content may be
hostile. Prompt injection turns "summarise this file" into "call the delete tool", and
the request looks perfectly well-formed at the protocol level.

| Permission | Meaning |
| --- | --- |
| `Read` | read something it already knows the identity of |
| `Search` | search across content |
| `Propose` | prepare a change for a human to confirm; nothing takes effect |
| `Commit` | apply a change without confirmation |
| `Delete` | remove content |

Default: read, search and propose. Two mechanics matter more than the list:

- Tools beyond the grant are **never listed**, not merely refused. Advertising a tool
  that always fails teaches the model to retry.
- A refused call **never reaches the tool**, and is logged.

The demo demonstrates both: it registers a `demo.threshold.set` tool with `Commit`
permission, which stays invisible and is refused, next to a `demo.threshold.propose`
tool that returns what *would* change and writes nothing.

## Embedded inference

For features where the application itself calls a model:

```rust
let completion = ai.complete(Prompt::new("Summarise this note", &note)).await?;
```

`Prompt` defaults to temperature zero and a token cap — most product features are
extraction or classification, where variation is a defect and an uncapped prompt is one
bad input away from a surprising bill.

`NoopAiService` fails rather than returning an empty answer: a feature that quietly
produces nothing is harder to diagnose than one that says it is unavailable.
`RecordingAiService` records prompts, so the prompt itself — the part most likely to be
wrong — is reviewable in a test without a network, a key or cost.

## Logging and stdout

When serving MCP over stdio, **stdout belongs to the protocol**. One log line corrupts
the stream and the client reports a parse error that points nowhere near logging:

```rust
origin_telemetry::init(TelemetryConfig::for_stdout_protocol());
```

## HTTP loopback & running instance attach

While stdio serves headless execution when the application is not running, desktop users
already have the GUI open. Spawning a second desktop process would collide with single-instance
locks and create split-brain database states.

Origin provides `origin-mcp-http` (ADR-0031):
- The running GUI binds an ephemeral HTTP loopback endpoint (`127.0.0.1:0`, path `/mcp`).
- The port and product metadata are written to an application discovery file (`mcp-http.json`).
- Access requires a 256-bit CSPRNG bearer token (`Authorization: Bearer <token>`), negotiated via
  a user confirmation prompt in the GUI (G19).
- If invoked via stdio while the GUI is running, the CLI transparently proxies standard I/O streams
  to the running GUI's loopback endpoint.

## Visibility and safety

- **Activity indication:** AI operations are surfaced to the user (e.g. tray badge/activity state).
- **Mutating operations:** Mutations must be gated by `ConfirmationService`. A default `DenyingConfirmationService`
  ensures unprompted mutations fail closed.
- **Audit logging:** Invocations and permission denials are recorded via telemetry.

## Open questions

- **Protocol revision.** The envelope was written against the stable core of MCP;
  verify it against the current specification before connecting a real client (→ Plan 05).
