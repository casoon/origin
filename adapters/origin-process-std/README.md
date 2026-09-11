# origin-process-std

Standard library process execution adapter for Origin (`ProcessRunner`, B1).

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_platform::{ProcessAllowlist, ProcessRunner};
use origin_process_std::StdProcessRunner;
use std::path::Path;

let allowlist = ProcessAllowlist::new(["git"]);
let runner = StdProcessRunner::new(allowlist);

let output = runner.run("git", &["status".to_owned()], Path::new(".")).await?;
assert!(output.success());
```

Runs external commands via `tokio::process`. Enforces the security boundary in Rust before
reaching the operating system: only binaries configured in the `ProcessAllowlist` are executed,
arguments are passed as distinct vectors (avoiding shell escaping vulnerabilities), and stdout/stderr
streams are bounded.

## Stability

Pre-1.0 (`0.2.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
