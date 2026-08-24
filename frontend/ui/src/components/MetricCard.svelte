<script lang="ts">
  import type { Metric } from "@origin/client";

  interface Props {
    label: string;
    metric: Metric | null;
    /** Relative change against the previous period, e.g. -0.4 for a 40 % drop. */
    changeRatio?: number | null;
  }

  let { label, metric, changeRatio = null }: Props = $props();

  function formatUnit(metric: Metric): string {
    if (typeof metric.unit === "object") return metric.unit.custom;
    switch (metric.unit) {
      case "percent":
        return "%";
      case "bytes":
        return "B";
      case "milliseconds":
        return "ms";
      case "per_minute":
        return "/min";
      default:
        return "";
    }
  }

  const value = $derived(
    metric ? `${metric.value.toLocaleString(undefined, { maximumFractionDigits: 1 })}` : "—",
  );
  const unit = $derived(metric ? formatUnit(metric) : "");
  const change = $derived(
    changeRatio === null ? null : `${changeRatio > 0 ? "+" : ""}${Math.round(changeRatio * 100)} %`,
  );
</script>

<div class="rounded-lg border border-border-subtle bg-surface p-4">
  <p class="text-xs font-medium tracking-wide text-muted uppercase">{label}</p>
  <p class="mt-2 flex items-baseline gap-1">
    <span class="text-3xl font-semibold tabular-nums">{value}</span>
    {#if unit}<span class="text-sm text-muted">{unit}</span>{/if}
  </p>
  {#if change}
    <p class="mt-1 text-xs {changeRatio! < 0 ? 'text-critical' : 'text-healthy'}">
      {change} vs. previous
    </p>
  {:else if metric}
    <p class="mt-1 text-xs text-muted">
      {new Date(metric.at).toLocaleTimeString()}
    </p>
  {/if}
</div>
