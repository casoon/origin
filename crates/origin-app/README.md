# origin-app

Composition root machinery for Origin applications: ApplicationBuilder, modules, service registry.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_app::{ApplicationBuilder, ApplicationModule, ModuleRegistry};
use origin_domain::Result;

#[derive(Debug)]
struct PulseModule;

impl ApplicationModule for PulseModule {
    fn id(&self) -> &'static str {
        "pulse"
    }

    fn register(&self, registry: &mut ModuleRegistry) -> Result<()> {
        // read settings, provide services, subscribe to events
        Ok(())
    }
}

let app = ApplicationBuilder::in_memory()
    .module(PulseModule)
    .build()?;

assert_eq!(app.modules(), &["pulse"]);
```

`ApplicationBuilder::in_memory()` wires in-memory storage, a memory secret store and a
no-op notification service — the configuration the whole application is tested against
without starting a desktop session.

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
