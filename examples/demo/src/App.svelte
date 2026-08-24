<script lang="ts">
  import {
    AlertList,
    AppShell,
    EmptyState,
    ErrorState,
    HealthIndicator,
    MetricCard,
    Wordmark,
  } from "@origin/ui";
  import {
    accounts as accountsApi,
    appInfo,
    connectors as connectorsApi,
    onPlatformEvent,
    openUrl,
    sync as syncApi,
    OriginError,
    settings,
    toOriginError,
    type Account,
    type AppInfo,
    type ConnectorDescriptor,
    type PlatformEvent,
    type ProductPermission,
    type SyncStatus,
  } from "@origin/client";
  import { pulse, type PulseSnapshot } from "./demo-client";
  import { onMount } from "svelte";

  const REPOSITORY_URL = "https://github.com/casoon/origin";

  let info = $state<AppInfo | null>(null);
  let snapshot = $state<PulseSnapshot | null>(null);
  let error = $state<OriginError | null>(null);
  let refreshing = $state(false);
  let lastEvent = $state<string | null>(null);
  let criticalThreshold = $state<number | null>(null);
  let connectors = $state<ConnectorDescriptor[]>([]);
  let accounts = $state<Account[]>([]);
  let syncStatus = $state<SyncStatus[]>([]);

  const alerts = $derived(snapshot?.alerts ?? []);

  function describePermission(permission: ProductPermission): string {
    return "read" in permission
      ? `read ${permission.read.scope}`
      : `write ${permission.write.scope}`;
  }

  async function load() {
    try {
      [info, snapshot, criticalThreshold, connectors, accounts, syncStatus] =
        await Promise.all([
          appInfo(),
          pulse.snapshot(),
          settings.get<number>("demo.critical_above"),
          connectorsApi(),
          accountsApi.list(),
          syncApi.status(),
        ]);
      error = null;
    } catch (thrown) {
      error = toOriginError(thrown);
    }
  }

  async function refresh() {
    refreshing = true;
    try {
      [snapshot, syncStatus] = await Promise.all([pulse.refresh(), syncApi.status()]);
      error = null;
    } catch (thrown) {
      error = toOriginError(thrown);
    } finally {
      refreshing = false;
    }
  }

  async function lowerThreshold() {
    // Demonstrates that settings live in Rust: the next refresh uses the new value.
    const next = 40;
    await settings.set("demo.critical_above", next);
    criticalThreshold = next;
    await refresh();
  }

  function describe(event: PlatformEvent): string {
    switch (event.type) {
      case "sync_completed":
        return `sync completed (${event.changed} changed)`;
      case "sync_failed":
        return `sync failed: ${event.message}`;
      case "alert_raised":
        return event.deduplicated
          ? `alert still active: ${event.alert.title}`
          : `alert raised: ${event.alert.title}`;
      case "alert_resolved":
        return "alert resolved";
      case "account_expired":
        return `account expired: ${event.account}`;
      case "job_started":
        return `job started: ${event.kind}`;
      case "job_progress":
        return event.total === null
          ? `job running: ${event.current}`
          : `job ${Math.round((event.current / event.total) * 100)} %`;
      case "job_finished":
        return `job ${event.status}: ${event.kind}`;
    }
  }

  onMount(() => {
    void load();

    // The background loop in Rust keeps running while this window is closed; when it
    // is open, the bridge tells us what happened.
    const subscription = onPlatformEvent((event) => {
      lastEvent = describe(event);
      void pulse.snapshot().then((next) => (snapshot = next));
      void syncApi.status().then((next) => (syncStatus = next));
    }).catch((thrown) => {
      // Subscribing needs the Tauri bridge; outside it (e.g. this window opened as a
      // plain page) that is a normal failure, not an unhandled rejection.
      error = toOriginError(thrown);
      return undefined;
    });

    return () => {
      void subscription.then((unlisten) => unlisten?.());
    };
  });
</script>

<AppShell
  title="Origin Demo"
  subtitle={info ? `${info.name} ${info.version} · modules: ${info.modules.join(", ")}` : "starting…"}
>
  {#snippet actions()}
    <button
      type="button"
      class="rounded-md border border-border-subtle px-3 py-1.5 text-sm hover:bg-surface-raised disabled:opacity-50"
      onclick={refresh}
      disabled={refreshing}
    >
      {refreshing ? "Refreshing…" : "Refresh"}
    </button>
  {/snippet}

  <div class="space-y-8">
    <div class="flex min-h-40 items-center justify-center">
      <Wordmark text="ORIGIN" />
    </div>

    {#if error}
      <ErrorState {error} onRetry={refresh} />
    {/if}

    <section class="space-y-3">
      <header class="flex items-center justify-between">
        <h2 class="text-sm font-semibold">Status</h2>
        <HealthIndicator health={snapshot?.health ?? "unknown"} />
      </header>

      <div class="grid gap-3 sm:grid-cols-2">
        <MetricCard label="Load" metric={snapshot?.metric ?? null} />
        <div class="rounded-lg border border-border-subtle bg-surface p-4">
          <p class="text-xs font-medium tracking-wide text-muted uppercase">
            Critical above
          </p>
          <p class="mt-2 text-3xl font-semibold tabular-nums">
            {criticalThreshold ?? 85}<span class="ml-1 text-sm text-muted">%</span>
          </p>
          <button
            type="button"
            class="mt-3 rounded-md border border-border-subtle px-3 py-1.5 text-xs hover:bg-surface-raised"
            onclick={lowerThreshold}
          >
            Lower to 40 % and refresh
          </button>
        </div>
      </div>
    </section>

    <section class="space-y-3">
      <h2 class="text-sm font-semibold">Alerts</h2>
      <AlertList {alerts} />
    </section>

    <section class="space-y-3">
      <h2 class="text-sm font-semibold">Sync</h2>
      <ul class="space-y-2">
        {#each syncStatus as status (status.target.name)}
          <li class="rounded-lg border border-border-subtle bg-surface p-4">
            <div class="flex items-baseline justify-between gap-4">
              <p class="font-medium">{status.target.name}</p>
              <HealthIndicator health={status.health} />
            </div>
            <p class="mt-1 text-sm text-muted">
              {#if status.state.failure_streak > 0}
                {status.state.failure_streak} consecutive failure(s) ·
              {/if}
              {#if status.due_at}
                next run {new Date(status.due_at).toLocaleTimeString()}
              {:else}
                not scheduled
              {/if}
            </p>
          </li>
        {/each}
      </ul>
    </section>

    <section class="space-y-3">
      <h2 class="text-sm font-semibold">Connections</h2>

      <ul class="space-y-2">
        {#each connectors as connector (connector.id)}
          <li class="rounded-lg border border-border-subtle bg-surface p-4">
            <div class="flex items-baseline justify-between gap-4">
              <p class="font-medium">{connector.display_name}</p>
              <span class="text-xs text-muted">
                {connector.auth === "none" ? "no sign-in required" : "OAuth"}
              </span>
            </div>
            <p class="mt-1 text-sm text-muted">
              Requests: {connector.required_permissions.map(describePermission).join(", ") ||
                "nothing"}
            </p>
          </li>
        {/each}
      </ul>

      {#if accounts.length === 0}
        <EmptyState
          title="No accounts connected"
          description="This demo talks to no external service, so it needs none."
        />
      {:else}
        <ul class="space-y-2">
          {#each accounts as account (account.id)}
            <li
              class="flex items-center justify-between rounded-lg border border-border-subtle bg-surface p-4"
            >
              <span>{account.display_name}</span>
              <span class="text-xs text-muted">{account.status}</span>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="space-y-2">
      <h2 class="text-sm font-semibold">Last platform event</h2>
      <p class="selectable font-mono text-xs text-muted">
        {lastEvent ?? "waiting for the background loop…"}
      </p>
    </section>
  </div>

  {#snippet footer()}
    <div class="flex items-center justify-between">
      <span>Close the window — the app keeps running in the tray.</span>
      <button
        type="button"
        class="underline underline-offset-2 hover:text-text"
        onclick={() => openUrl(REPOSITORY_URL)}
      >
        Repository
      </button>
    </div>
  {/snippet}
</AppShell>
