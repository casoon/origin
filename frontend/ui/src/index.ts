/**
 * `@origin/ui` — the shared component layer.
 *
 * Products compose these and add their own components. They do not fork this package
 * (ADR-0013); a missing variant is a change here, not a copy there.
 */

export { default as AppShell } from "./components/AppShell.svelte";
export { default as AlertList } from "./components/AlertList.svelte";
export { default as EmptyState } from "./components/EmptyState.svelte";
export { default as ErrorState } from "./components/ErrorState.svelte";
export { default as HealthIndicator } from "./components/HealthIndicator.svelte";
export { default as MetricCard } from "./components/MetricCard.svelte";
export { default as Wordmark } from "./components/Wordmark.svelte";
