//! Origin's own maintenance entry point.
//!
//! Every derivative's `xtask` looks exactly like this — the tasks themselves live in
//! `origin-xtask`, so they arrive with a version bump instead of being copied.

fn main() -> std::process::ExitCode {
    origin_xtask::main()
}
