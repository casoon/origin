//! Three lines on purpose.
//!
//! The tasks — architecture validation, generation, migrations, the CI recipe — live
//! in `origin-xtask`. They arrive with a version bump instead of being copied here and
//! drifting apart from every other Origin project.

fn main() -> std::process::ExitCode {
    origin_xtask::main()
}
