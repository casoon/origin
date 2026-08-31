# origin-manifest

The Origin app manifest: what a product is, parsed and validated.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_manifest::Manifest;

let manifest = Manifest::load("app.toml")?;

println!("{} v{}", manifest.product.name, manifest.product.version);
println!("modules enabled: {:?}", manifest.enabled_modules());
```

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
