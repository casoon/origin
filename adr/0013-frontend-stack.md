# ADR-0013  Frontend Stack: Svelte 5 + Vite + Tailwind v4

Status:   Accepted
Date:     2026-08-23

## Context

Origin needs one frontend baseline for `@origin/ui` and all products. Desktop apps
are sensitive to bundle size and startup time, and the shell is long-lived UI rather
than a document.

## Decision

Svelte 5 (Runes) + Vite + Tailwind v4 (CSS-first `@theme`).

- `@origin/client` is framework-free TypeScript — it must stay usable from any UI.
- `@origin/ui` is the Svelte component library.
- Products may add their own components; they may not fork `@origin/ui`.

## Consequences

- Small bundles, no virtual DOM, fast cold start.
- Smaller component ecosystem than React — `@origin/ui` carries more weight.
- A future React product would reuse `@origin/client` but not `@origin/ui`.
