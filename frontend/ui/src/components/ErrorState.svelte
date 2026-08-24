<script lang="ts">
  import { OriginError } from "@origin/client";

  interface Props {
    error: OriginError;
    onRetry?: () => void;
  }

  let { error, onRetry }: Props = $props();

  /**
   * One message per error kind, so every Origin app explains the same failure the
   * same way. The raw message is shown as detail, never as the headline.
   */
  const headline: Record<OriginError["kind"], string> = {
    authentication: "Your session has expired",
    permission: "This action is not permitted",
    network: "Could not reach the service",
    offline: "You are offline",
    rate_limited: "The service is rate limiting us",
    storage: "Local data could not be read",
    external_service: "The service reported a problem",
    validation: "That input was not accepted",
    configuration: "This application is misconfigured",
    internal: "Something went wrong",
  };
</script>

<div class="rounded-lg border border-critical/40 bg-surface p-4">
  <p class="text-sm font-medium text-critical">{headline[error.kind]}</p>
  <p class="mt-1 text-sm text-muted">{error.message}</p>
  {#if error.retryAfterSeconds}
    <p class="mt-1 text-xs text-muted">Try again in {error.retryAfterSeconds} s.</p>
  {/if}
  {#if error.retryable && onRetry}
    <button
      type="button"
      class="mt-3 rounded-md border border-border-subtle px-3 py-1.5 text-sm hover:bg-surface-raised"
      onclick={onRetry}
    >
      Try again
    </button>
  {/if}
</div>
