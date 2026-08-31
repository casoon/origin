# origin-domain

Origin domain primitives and platform ports. Knows no product, no Tauri, no storage engine.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_domain::{AppError, Clock, ErrorKind, SystemClock};

let error = AppError::validation("account id must not be empty");
assert_eq!(error.kind(), ErrorKind::Validation);
assert!(!error.is_retryable());

let clock = SystemClock;
let now = clock.now();
```

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
