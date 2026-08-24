# ADR-0029  The AI Permission Level

Status:   Accepted
Date:     2026-08-24

## Context

A tool call arrives from a language model that is acting on content it read somewhere —
a document, an issue, a web page. That content may be hostile. Prompt injection turns
"summarise this file" into "call the delete tool", and the request looks perfectly
well-formed at the protocol level.

The existing two levels do not cover it: product permissions say what the application
may do at a service, platform permissions what it may do on this machine. Neither says
what an *external agent* may do with the application.

## Decision

A third permission level, never mixed with the other two:

| Permission | Meaning |
| --- | --- |
| `Read` | read something it already knows the identity of |
| `Search` | search across content |
| `Propose` | prepare a change for a human to confirm; nothing takes effect |
| `Commit` | make a change take effect without confirmation |
| `Delete` | remove content |

**The default grant is read, search and propose.** Nothing an external AI does takes
effect without a human.

Two mechanics matter more than the list:

- **Tools beyond the grant are not listed at all**, not merely refused on call.
  Advertising a tool that always fails wastes the model's attempts and teaches it to
  retry.
- **A refused call never reaches the tool**, and is logged. An external agent
  repeatedly reaching for a permission it does not have is worth seeing.

The grant is never wider than the rights of the signed-in user. MCP must not be a
privilege escalation.

Write access is therefore modelled as *propose*, not *write*: the tool returns what
would change, and a human confirms it in the application.

## Consequences

- Switching on `Commit` is a deliberate act, visible in one place, and a settings screen
  can warn about it.
- Products must express mutations as proposals to be useful under the default grant.
  That is more work, and it is the right amount of work.
- A tool that needs a permission the product does not grant is invisible rather than
  broken — which is the correct failure mode for a caller that cannot ask why.
