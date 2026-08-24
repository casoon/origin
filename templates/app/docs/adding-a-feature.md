# Adding a feature

Use `src-tauri/src/example.rs` as the worked example. Four steps, in this order.

## 1. The service

The logic lives here, and it takes everything it needs from `Platform`:

```rust
#[derive(Debug)]
pub struct InboxService {
    platform: Platform,
}

impl InboxService {
    pub async fn unread(&self) -> Result<Vec<Message>> {
        self.platform.cache.get(&key).await
    }
}
```

It must not open a database, call an OS API, or hold an `AppHandle`. If it needs
something the platform does not offer, that is a platform gap worth raising — not
something to work around locally.

## 2. The module

```rust
impl ApplicationModule for InboxModule {
    fn id(&self) -> &'static str { "inbox" }

    fn register(&self, registry: &mut ModuleRegistry) -> Result<()> {
        registry.provide(Arc::new(InboxService::new(registry.platform().clone())));
        Ok(())
    }
}
```

Register it in the composition root and switch it on in `app.toml`:

```toml
[modules]
inbox = true
```

A module that fails to register fails startup, with its name in the error. A
misconfigured module must not start half-working.

## 3. Settings, if it has any

Key and default declared together, next to the code that reads them:

```rust
const REFRESH_MINUTES: Setting<u32> = Setting::new("inbox.refresh_minutes", || 5);
```

A stored value that no longer decodes — because the type changed between releases —
falls back to the default with a warning rather than blocking startup.

## 4. The command and its client wrapper

The command resolves, delegates and translates. No logic:

```rust
#[tauri::command]
pub async fn inbox_unread(state: State<'_, OriginState>) -> Result<Vec<Message>, CommandError> {
    Ok(state.application().require::<InboxService>()?.unread().await?)
}
```

Add it to the handler list in `lib.rs`, then wrap it in `ui/src/client.ts`:

```ts
export function unread(): Promise<Message[]> {
  return command<Message[]>("inbox_unread");
}
```

No component ever calls `command()` with a raw string, and none imports
`@tauri-apps/api`. `cargo xtask validate` fails the build on either — including on a
command name that no `#[tauri::command]` defines, which is otherwise a runtime-only
mistake.

## 5. Test it without Tauri

```rust
#[tokio::test]
async fn unread_messages_come_from_the_cache() {
    let application = ApplicationBuilder::in_memory()
        .module(InboxModule)
        .build()
        .unwrap();

    let service = application.require::<InboxService>().unwrap();
    assert!(service.unread().await.unwrap().is_empty());
}
```

If you cannot write this test, the module knows too much about the host.

## Background work

**Something on a schedule** — a poll, a refresh — is a sync target. Implement
`SyncSource` and register it with a policy; the engine owns retry, exponential backoff,
jitter, offline handling and single-flight, so your source only answers *how* to fetch:

```rust
platform.sync.register(
    SyncTarget::new(connector, account, "inbox"),
    SyncPolicy::every(Duration::minutes(5)),
    Arc::new(InboxSource::new(api)),
);
```

**Something long-running** — an export, a report, a scan — is a job:

```rust
platform.jobs.spawn("export", |ctx| async move {
    for (index, item) in items.iter().enumerate() {
        if ctx.is_cancelled() { return Ok(()); }
        ctx.progress(index as u64 + 1, Some(items.len() as u64)).await;
    }
    Ok(())
});
```

Progress and cancellation are already wired to the UI.

## Reacting to things

Need a result now? Call the service. Might several independent parts react? Publish a
typed event:

```rust
platform.events.publish(PlatformEvent::AlertRaised(AlertRaised { alert, deduplicated }));
```

The publisher does not decide whether that becomes a notification, a UI update or
nothing.
