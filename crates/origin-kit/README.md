# origin-kit

The `origin` command: create a new Origin application from the embedded project
template, without a checkout of Origin.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```bash
cargo install origin-kit
origin new my-app --name "My App" --id dev.example.myapp
```

The generated project depends on the published `origin-*` crates and `@casoon/origin-*`
packages by registry version, and carries its own `xtask` for the tasks that belong to
a project rather than to this tool:

```bash
cd my-app
cargo xtask validate    # the architecture rules
cargo xtask generate    # the files derived from app.toml
cargo xtask update      # migrations between Origin versions
```

## The template

The template is compiled into the binary, so `origin new` needs nothing but this crate.
Its three manifests are stored as `Cargo.toml.in`: `cargo package` skips any directory
that holds a `Cargo.toml`, which would leave the published crate without a template at
all. The suffix is stripped when a project is written, and a test fails if a template
manifest ever appears under its real name (ADR-0032).

## Stability

Pre-1.0 (`0.2.1`). The command-line interface and the public functions may still change
between minor versions; pin an exact version if that matters to you.
