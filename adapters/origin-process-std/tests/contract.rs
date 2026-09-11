//! The shared `ProcessRunner` contract, run against the real machine.

use origin_platform::ProcessAllowlist;
use origin_process_std::StdProcessRunner;

/// The contract invokes the allowed program with `["--version"]`, which only has a
/// predictable, side-effect-free meaning for the tools used here on Unix. The refusal
/// path — the part that actually guards the OS — is unit-tested on every platform.
#[cfg(unix)]
#[tokio::test]
async fn std_process_runner_satisfies_the_process_contract() {
    let allowlist = ProcessAllowlist::new(["uname"]);
    let runner = StdProcessRunner::new(allowlist.clone());

    origin_platform::process_contract::run_all(&runner, &allowlist).await;
}

#[cfg(not(unix))]
#[test]
fn the_process_contract_requires_a_unix_like_program_with_version_output() {
    // Documented no-op on Windows; see the module comment above.
}
