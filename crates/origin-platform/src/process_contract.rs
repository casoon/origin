//! The contract every [`ProcessRunner`] implementation must satisfy (ADR-0004, §41).
//!
//! Adapters run this suite against their own backend so that an in-memory double
//! and a real Tauri process runner cannot drift apart.
//!
//! ```ignore
//! #[tokio::test]
//! async fn satisfies_the_process_runner_contract() {
//!     let allowlist = origin_platform::ProcessAllowlist::new(["git"]);
//!     let runner = MemoryProcessRunner::new(allowlist.clone(), ProcessOutput { status: 0, stdout: vec![], stderr: vec![] });
//!     origin_platform::process_contract::run_all(&runner, &allowlist).await;
//! }
//! ```

use crate::process::{ProcessAllowlist, ProcessRunner};
use crate::workspace::WorkspaceRoot;

/// Run every contract check against `runner`.
///
/// `allowlist` must be the same one the runner was configured with, so the
/// contract can pick an allowed program and construct one that is definitely
/// not allowed. Configure a denied argument on it to have that checked too.
pub async fn run_all<R: ProcessRunner>(runner: &R, allowlist: &ProcessAllowlist) {
    rejects_an_unlisted_program(runner).await;
    runs_an_allowlisted_program(runner, allowlist).await;
    rejects_a_denied_argument(runner, allowlist).await;
}

/// A program name that no product would ever allow — our guaranteed-disallowed
/// canary across every contract run.
const NEVER_ALLOWED: &str = "__origin_contract_never_allowed__";

async fn rejects_an_unlisted_program<R: ProcessRunner>(runner: &R) {
    let workspace_root =
        WorkspaceRoot::new(std::env::temp_dir()).expect("the temp dir is absolute");

    let result = runner
        .run(NEVER_ALLOWED, &[], workspace_root.as_path())
        .await;

    match result {
        Err(err) => {
            assert_eq!(
                err.kind(),
                origin_domain::ErrorKind::Permission,
                "a program not in the allowlist must be rejected with Permission, \
                 not a different error kind"
            );
        }
        Ok(_) => panic!(
            "an unlisted program `{NEVER_ALLOWED}` must be rejected before it \
             reaches the operating system"
        ),
    }
}

async fn runs_an_allowlisted_program<R: ProcessRunner>(runner: &R, allowlist: &ProcessAllowlist) {
    let allowed = allowlist
        .entries()
        .first()
        .expect("the allowlist must contain at least one program for the contract test");

    let workspace_root =
        WorkspaceRoot::new(std::env::temp_dir()).expect("the temp dir is absolute");

    let result = runner
        .run(allowed, &["--version".to_owned()], workspace_root.as_path())
        .await;

    match result {
        Ok(output) => {
            // The contract does not prescribe what a successful run looks like —
            // that belongs to the implementation. It only requires that listing
            // is honoured.
            let _ = output;
        }
        Err(err) => {
            // An allowed program may legitimately fail at runtime, but *not* with
            // a Permission error.
            assert_ne!(
                err.kind(),
                origin_domain::ErrorKind::Permission,
                "an allowlisted program `{allowed}` must not be rejected with Permission"
            );
        }
    }
}

/// An implementation that only checked the program name would pass the two checks
/// above; this one catches it.
async fn rejects_a_denied_argument<R: ProcessRunner>(runner: &R, allowlist: &ProcessAllowlist) {
    let Some((program, argument)) = allowlist.entries().iter().find_map(|program| {
        allowlist
            .denied_arguments(program)
            .first()
            .map(|argument| (program, argument))
    }) else {
        return;
    };

    let workspace_root =
        WorkspaceRoot::new(std::env::temp_dir()).expect("the temp dir is absolute");

    let result = runner
        .run(
            program,
            std::slice::from_ref(argument),
            workspace_root.as_path(),
        )
        .await;

    match result {
        Err(err) => assert_eq!(
            err.kind(),
            origin_domain::ErrorKind::Permission,
            "a denied argument must be rejected with Permission"
        ),
        Ok(_) => panic!(
            "`{program} {argument}` is denied and must be rejected before it reaches the \
             operating system"
        ),
    }
}
