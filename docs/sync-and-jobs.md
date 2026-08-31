# Sync and jobs

Decisions: [ADR-0017](../adr/0017-sync-engine-owns-scheduling.md),
[ADR-0018](../adr/0018-backoff-jitter-and-offline.md),
[ADR-0019](../adr/0019-storage-namespace-convention.md),
[ADR-0020](../adr/0020-jobs-are-process-local.md).

## The split

```text
A SyncSource decides HOW data is fetched.
The engine decides WHEN, and under which conditions.
```

A source implements one method and owns none of the timing:

```rust
#[async_trait]
impl SyncSource for NotificationsSource {
    async fn sync(&self, context: &SyncContext) -> Result<SyncResult> {
        let response = self.api.notifications(context.etag()).await?;
        if response.not_modified {
            return Ok(SyncResult::NotModified);
        }
        self.store(response.items).await?;
        Ok(SyncResult::Updated(
            SyncReport::changed(response.items.len() as u64).with_etag(response.etag),
        ))
    }
}
```

Register it with a policy:

```rust
platform.sync.register(
    SyncTarget::new(connector, account, "notifications"),
    SyncPolicy::every(Duration::minutes(1)),
    Arc::new(NotificationsSource::new(api)),
);
```

Same engine for a one-minute notification poll and a six-hour analytics refresh — only
the numbers differ.

## Three ways a sync starts

| Entry point | Who calls it | Throttled? |
| --- | --- | --- |
| `run_due(now)` | the scheduler | follows the policy |
| `sync_if_due(target)` | automatic triggers — window focus, network returning | yes, by `min_interval` |
| `sync_now(target)` | the user pressed Refresh | no |

The distinction matters: alt-tabbing twenty times must not mean twenty syncs, but a
user who explicitly asked for a refresh should get one.

## Failure handling

- **Backoff** is exponential with a cap and ±20 % jitter, so several targets that failed
  together do not retry in lockstep against a recovering service.
- **Offline** gets a flat, short retry instead. Connectivity usually returns in one
  step; exponential backoff would leave the app stale long after the network came back.
  There is no `NetworkStatus` port — a link is not the same as reachability, so the
  attempt itself is the probe (ADR-0018).
- **Validators survive.** A response that omits its ETag does not clear the stored one.
  Dropping it would turn every later sync into a full refetch.
- **Single-flight**: a second caller waits for the run in flight rather than starting a
  parallel one, which would race on the validators.

## Testing scheduling without waiting

`run_due(now)` does one pass for a given instant, and the background loop is a thin
wrapper around it. So a test moves a fake clock instead of sleeping:

```rust
harness.engine.run_due(clock.now()).await;
clock.advance(Duration::minutes(6));
harness.engine.run_due(clock.now()).await;
```

This is why backoff takes its random value as an argument: `delay_for(failures, random)`
is a pure function, and jitter is otherwise untestable.

## Health

`health_of(state, policy, now)` turns sync bookkeeping into the shared
[`Health`](../crates/origin-domain/src/health.rs) model. It is not a method on `SyncState`
because what counts as healthy depends on the cadence, and only the policy knows that.

A target that quietly stopped running is reported as `Warning`, not `Healthy` — silence
is not success.

## Jobs

For work that is not a sync: an export, a report render, a repository scan.

```rust
let id = jobs.spawn("export", |ctx| async move {
    for (index, item) in items.iter().enumerate() {
        if ctx.is_cancelled() {
            return Ok(());
        }
        ctx.progress(index as u64 + 1, Some(items.len() as u64)).await;
        write(item).await?;
    }
    Ok(())
});
```

- **Cancellation is cooperative.** `cancel` records the request; the job stops when it
  can do so safely.
- **A panicking job is recorded as failed**, not lost. A job the UI shows as running
  forever is worse than one reporting an error.
- **Progress is throttled** to roughly one event per percent. Reporting all ten thousand
  steps would flood the bus and push slow subscribers into lag.
- **Jobs are process-local** (ADR-0020). Nothing survives a restart, and nothing pretends
  to.

## Where the state lives

Sync state is stored under the account prefix (ADR-0019):

```text
acct.<connector>.<account>.sync
```

So disconnecting an account removes its sync bookkeeping along with everything else it
owned — no module has to register anything for that to work.
