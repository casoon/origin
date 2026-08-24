<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    subtitle?: string;
    /** Buttons and controls shown in the header, right-aligned. */
    actions?: Snippet;
    footer?: Snippet;
    children: Snippet;
  }

  let { title, subtitle, actions, footer, children }: Props = $props();
</script>

<div class="flex min-h-screen flex-col bg-canvas text-text">
  <header
    class="sticky top-0 z-10 flex items-center gap-4 border-b border-border-subtle bg-surface/90 px-6 py-4 backdrop-blur"
  >
    <div class="min-w-0 flex-1">
      <h1 class="truncate text-base font-semibold tracking-tight">{title}</h1>
      {#if subtitle}
        <p class="truncate text-xs text-muted">{subtitle}</p>
      {/if}
    </div>
    {#if actions}
      <div class="flex shrink-0 items-center gap-2">{@render actions()}</div>
    {/if}
  </header>

  <main class="mx-auto w-full max-w-4xl flex-1 px-6 py-8">
    {@render children()}
  </main>

  {#if footer}
    <footer class="border-t border-border-subtle px-6 py-3 text-xs text-muted">
      {@render footer()}
    </footer>
  {/if}
</div>
