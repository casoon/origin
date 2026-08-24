<script lang="ts">
  import type { Health } from "@origin/client";

  interface Props {
    health: Health;
    /** Show the state as text next to the dot. */
    label?: boolean;
  }

  let { health, label = true }: Props = $props();

  const presentation: Record<Health, { dot: string; text: string; label: string }> = {
    healthy: { dot: "bg-healthy", text: "text-healthy", label: "Healthy" },
    warning: { dot: "bg-warning", text: "text-warning", label: "Warning" },
    critical: { dot: "bg-critical", text: "text-critical", label: "Critical" },
    unknown: { dot: "bg-unknown", text: "text-unknown", label: "Unknown" },
  };

  const current = $derived(presentation[health]);
</script>

<span class="inline-flex items-center gap-2 text-sm font-medium {current.text}">
  <span class="size-2.5 rounded-full {current.dot}" aria-hidden="true"></span>
  {#if label}{current.label}{/if}
  <span class="sr-only">Status: {current.label}</span>
</span>
