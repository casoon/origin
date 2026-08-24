# Creating a module

A module is a feature area: Inbox, Projects, Traffic, Health. Use
[`examples/demo/src-tauri/src/pulse.rs`](../examples/demo/src-tauri/src/pulse.rs) as the
worked example.

## 1. Write the service

The service holds the logic and takes everything it needs from `Platform`:

```rust
pub struct InboxService {
    platform: Platform,
}
```

It must not open a database, call an OS API, or hold an `AppHandle`. If it needs
something the platform does not offer, that is a platform gap — raise it, do not work
around it.

## 2. Register it

```rust
impl ApplicationModule for InboxModule {
    fn id(&self) -> &'static str { "inbox" }

    fn register(&self, registry: &mut ModuleRegistry) -> Result<()> {
        registry.provide(Arc::new(InboxService::new(registry.platform().clone())));
        Ok(())
    }
}
```

Registration failure is a startup failure with the module named in the error — a
misconfigured module must not start half-working.

## 3. Declare settings next to the code that reads them

```rust
const REFRESH_MINUTES: Setting<u32> = Setting::new("inbox.refresh_minutes", || 5);
```

Key and default in one place. A stored value that no longer decodes falls back to the
default with a warning rather than blocking startup.

## 4. Publish events for anything others may react to

```rust
platform.events.publish(PlatformEvent::AlertRaised(AlertRaised { alert, deduplicated }));
```

The module does not decide whether that becomes a notification, a UI update or nothing.

## 5. Add commands and a client wrapper

A command resolves state, delegates and translates errors — no logic:

```rust
#[tauri::command]
pub async fn inbox_list(state: State<'_, OriginState>) -> Result<Vec<Item>, CommandError> {
    Ok(state.application().require::<InboxService>()?.list().await?)
}
```

Then a typed wrapper in the product's client slice, so no component ever calls
`command()` with a raw string:

```ts
export const inbox = {
  list: () => command<Item[]>("inbox_list"),
};
```

## 6. Test it without Tauri

Build an in-memory application, register the module, assert on behaviour. If you cannot,
something in the module knows too much about the host.

## Before you promote anything to `crates/`

Read [ADR-0009](../adr/0009-module-promotion-rule.md). A feature stays in the product
until a genuinely neutral abstraction has emerged — in practice at the third occurrence,
not the first similarity.
