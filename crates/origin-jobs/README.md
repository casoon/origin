# origin-jobs

Background jobs for Origin: progress, cancellation and a uniform lifecycle.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_domain::Clock;
use origin_events::EventBus;
use origin_jobs::Jobs;
use std::sync::Arc;

let jobs = Jobs::new(EventBus::new(), clock);

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

let status = jobs.get(&id).await;
jobs.cancel(&id).await?;
```

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
