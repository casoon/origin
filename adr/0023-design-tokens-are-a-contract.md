# ADR-0023  Design Tokens are a Stable Contract

Status:   Accepted
Date:     2026-08-23

## Context

Token names end up in every component's class list: `bg-surface`, `text-muted`,
`border-border-subtle`. Renaming one after four derivatives exist is a find-and-replace
across four repositories, in files Origin does not own.

That makes token names one of the few frontend decisions that is expensive to change
late — the same category as the storage namespace layout (ADR-0019).

## Decision

The token names in `@origin/ui`'s `theme.css` are a stable contract, frozen at this
set:

```text
colour     canvas · surface · surface-raised · border-subtle · text · muted · accent
state      healthy · warning · critical · unknown
accent     silver
type       font-sans · font-mono
```

- **Names are semantic, never literal.** `surface`, not `gray-100`. A literal name is a
  promise about a value, and it breaks the moment the value changes for dark mode.
- **Values may change freely**; names may not. Adding a token is additive and cheap;
  renaming or removing one is a breaking change for every derivative and needs an ADR.
- **Light and dark are two value sets for the same names**, so no component branches on
  the colour scheme.
- Raw hex in product code is a defect: it is exactly what makes two Origin applications
  drift apart visually.

## Consequences

- A product can restyle itself entirely by overriding values, without touching markup.
- The palette cannot be extended casually — a new token is a platform change.
- Adding a genuinely new concept (a `danger` surface, say) is allowed and additive.
  Renaming `muted` is not.
