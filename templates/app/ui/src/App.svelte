<script lang="ts">
  import { AppShell, EmptyState, ErrorState } from "@origin/ui";
  import { appInfo, OriginError, toOriginError, type AppInfo } from "@origin/client";
  import { greeting } from "./client";
  import { onMount } from "svelte";

  let info = $state<AppInfo | null>(null);
  let message = $state<string | null>(null);
  let error = $state<OriginError | null>(null);

  async function load() {
    try {
      [info, message] = await Promise.all([appInfo(), greeting()]);
      error = null;
    } catch (thrown) {
      error = toOriginError(thrown);
    }
  }

  onMount(() => {
    void load();
  });
</script>

<AppShell
  title={info?.name ?? "__PRODUCT_NAME__"}
  subtitle={info ? `${info.version} · modules: ${info.modules.join(", ")}` : "starting…"}
>
  <div class="space-y-6">
    {#if error}
      <ErrorState {error} onRetry={load} />
    {:else if message}
      <p class="text-2xl font-semibold">{message}</p>
      <EmptyState
        title="Your product starts here"
        description="Replace the example module in src-tauri/src/example.rs."
      />
    {/if}
  </div>
</AppShell>
