# @casoon/origin-ui

Shared Svelte 5 components and design tokens for
[Origin](https://github.com/casoon/origin) applications.

```ts
import { AppShell, EmptyState, ErrorState } from "@casoon/origin-ui";
```

```css
@import "@casoon/origin-ui/theme.css";
```

The token names in `theme.css` are a stable contract (ADR-0023). Products compose these
components and add their own; they do not fork this package (ADR-0013).

The package ships Svelte and TypeScript sources and expects a bundler that compiles them
(Vite with `@sveltejs/vite-plugin-svelte` in every Origin project).

License: MIT
