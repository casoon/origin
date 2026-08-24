# ADR-0028  The AI Provider Port

Status:   Accepted
Date:     2026-08-24

## Context

Some features need the application itself to call a model — summarise this note,
classify this message. That is not what MCP does (ADR-0027), and calling a provider SDK
from a module would put a vendor, a key and a network dependency into domain code.

## Decision

Inference is a port: `AiService` in `origin-ai`, implemented by `adapters/origin-ai-*`.

- Domain code never names a provider. Switching one, or letting the user pick, changes
  the composition root and nothing else.
- Errors use the normal model, so an unavailable model behaves like an unavailable API:
  outage is `ExternalService`, a rejected key is `Authentication`, a quota is
  `RateLimited`.
- `NoopAiService` **fails** rather than returning an empty answer. A feature that
  quietly produces nothing is harder to diagnose than one that says it is unavailable.
- A prompt carries an explicit `max_output_tokens` and a temperature that defaults to
  zero. Most product features are extraction or classification, where variation is a
  defect, and an uncapped prompt is one bad input away from a surprising bill.
- `RecordingAiService` records prompts, so the prompt itself — the part most likely to
  be wrong — is reviewable in a test without a network, a key or cost.

**No feature may depend on a model being reachable.** A product with the capability
switched off is a product with those features off, not a broken one.

## Consequences

- AI features are testable offline and deterministically.
- Usage is reported per completion, so a product can account for what it consumed.
- The port is intentionally narrow — one instruction over one input. Anything genuinely
  conversational needs a wider port and its own ADR.
