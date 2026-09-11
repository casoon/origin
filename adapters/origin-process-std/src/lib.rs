//! The process contract (`ProcessRunner`) over the local machine (B1).
//!
//! The allowlist is the security boundary, and it is enforced here in Rust *before*
//! anything reaches the operating system — exactly what the shared contract test
//! checks. There is no general shell: a program that is not configured is refused
//! with a permission error, and the arguments are passed as a vector, never through a
//! shell string, so there is nothing to quote-escape.

use async_trait::async_trait;
use origin_domain::{AppError, Result};
use origin_platform::{ProcessAllowlist, ProcessOutput, ProcessRunner};
use std::path::Path;
use tokio::process::Command;

/// Largest amount of `stdout`/`stderr` we keep. A runaway program must not be able to
/// grow the application's memory without bound.
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;

/// Runs allowlisted programs with `tokio::process`.
///
/// The allowlist is a field, not a parameter: the runner is constructed with the set
/// of programs a product permits, and that set is fixed for the runner's lifetime —
/// the same "named options, not a free list" posture as the security profiles.
#[derive(Debug, Clone)]
pub struct StdProcessRunner {
    allowlist: ProcessAllowlist,
}

impl StdProcessRunner {
    pub fn new(allowlist: ProcessAllowlist) -> Self {
        Self { allowlist }
    }

    pub fn allowlist(&self) -> &ProcessAllowlist {
        &self.allowlist
    }
}

#[async_trait]
impl ProcessRunner for StdProcessRunner {
    async fn run(&self, program: &str, args: &[String], cwd: &Path) -> Result<ProcessOutput> {
        // The gate. Anything the product did not configure stops here.
        self.allowlist.check(program)?;

        let output = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .output()
            .await
            .map_err(|error| {
                AppError::ExternalService(format!("cannot run `{program}`: {error}"))
            })?;

        let status = output.status.code().unwrap_or(-1);

        // Truncate rather than fail: a program that printed too much still ran, and the
        // caller usually only needs the exit code.
        let stdout = truncate(output.stdout);
        let stderr = truncate(output.stderr);

        Ok(ProcessOutput {
            status,
            stdout,
            stderr,
        })
    }
}

fn truncate(mut bytes: Vec<u8>) -> Vec<u8> {
    if bytes.len() > MAX_OUTPUT_BYTES {
        tracing::warn!(
            captured = bytes.len(),
            limit = MAX_OUTPUT_BYTES,
            "process output truncated"
        );
        bytes.truncate(MAX_OUTPUT_BYTES);
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A benign program present on the platform, with arguments that exit cleanly.
    ///
    /// `uname` takes no arguments and exits 0. On Windows `cmd` without arguments opens
    /// an interactive prompt, so `/C echo` is used instead — passing the arguments the
    /// way a product would.
    fn probe() -> (&'static str, Vec<String>) {
        #[cfg(unix)]
        {
            ("uname", Vec::new())
        }
        #[cfg(windows)]
        {
            (
                "cmd",
                vec!["/C".to_owned(), "echo".to_owned(), "ok".to_owned()],
            )
        }
    }

    #[tokio::test]
    async fn an_unlisted_program_never_reaches_the_operating_system() {
        let (program, _) = probe();
        let runner = StdProcessRunner::new(ProcessAllowlist::new([program]));

        let error = runner
            .run("definitely-not-allowed", &[], Path::new("."))
            .await
            .unwrap_err();

        assert_eq!(error.kind(), origin_domain::ErrorKind::Permission);
    }

    #[tokio::test]
    async fn an_allowlisted_program_runs_and_reports_its_output() {
        let (program, args) = probe();
        let runner = StdProcessRunner::new(ProcessAllowlist::new([program]));

        let output = runner
            .run(program, &args, Path::new("."))
            .await
            .expect("an allowlisted program must run");

        assert_eq!(
            output.status,
            0,
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!output.stdout.is_empty(), "the program printed its output");
    }

    #[tokio::test]
    async fn a_nonzero_exit_is_reported_not_treated_as_an_error() {
        // A program that fails is not a *runner* failure: the exit code is the answer.
        let (program, _) = probe();
        let runner = StdProcessRunner::new(ProcessAllowlist::new([program]));

        #[cfg(unix)]
        let args = vec!["--definitely-not-a-real-flag".to_owned()];
        #[cfg(windows)]
        let args = vec!["/C".to_owned(), "exit".to_owned(), "3".to_owned()];

        let output = runner
            .run(program, &args, Path::new("."))
            .await
            .expect("a non-zero exit still yields a ProcessOutput");

        assert_ne!(output.status, 0);
    }
}
