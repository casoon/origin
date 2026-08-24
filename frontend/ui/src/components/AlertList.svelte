<script lang="ts">
  import type { Alert } from "@origin/client";
  import EmptyState from "./EmptyState.svelte";

  interface Props {
    alerts: Alert[];
    emptyTitle?: string;
    emptyDescription?: string;
  }

  let {
    alerts,
    emptyTitle = "No active alerts",
    emptyDescription = "Everything is within its thresholds.",
  }: Props = $props();

  const severityClass: Record<Alert["severity"], string> = {
    info: "border-l-unknown",
    warning: "border-l-warning",
    critical: "border-l-critical",
  };
</script>

{#if alerts.length === 0}
  <EmptyState title={emptyTitle} description={emptyDescription} />
{:else}
  <ul class="space-y-2">
    {#each alerts as alert (alert.id)}
      <li
        class="rounded-lg border border-border-subtle border-l-4 bg-surface p-4 {severityClass[
          alert.severity
        ]}"
      >
        <div class="flex items-baseline justify-between gap-4">
          <p class="font-medium">{alert.title}</p>
          <time class="shrink-0 text-xs text-muted" datetime={alert.raised_at}>
            {new Date(alert.raised_at).toLocaleTimeString()}
          </time>
        </div>
        {#if alert.body}
          <p class="mt-1 text-sm text-muted">{alert.body}</p>
        {/if}
      </li>
    {/each}
  </ul>
{/if}
