# origin-xtask

Origin maintenance tasks as a library, so a derivative's xtask is three lines and its rules arrive with a version bump.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

A derivative's own `xtask/src/main.rs` is three lines:

```rust
fn main() -> std::process::ExitCode {
    origin_xtask::main()
}
```

`origin_xtask::main()` dispatches on the first process argument (`validate`, `generate`,
`update`, `new`, `ci`, `demo`) and returns the process exit code.

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
