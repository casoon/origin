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
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

/// Largest amount of `stdout`/`stderr` we keep. A runaway program must not be able to
/// grow the application's memory without bound.
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;

/// How long a program may run before it is stopped, unless configured otherwise.
///
/// Generous enough for a `git clone` of a large repository. The point is only that a
/// hung program cannot hang its caller forever.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Runs allowlisted programs with `tokio::process`.
///
/// The allowlist is a field, not a parameter: the runner is constructed with the set
/// of programs a product permits, and that set is fixed for the runner's lifetime —
/// the same "named options, not a free list" posture as the security profiles.
#[derive(Debug, Clone)]
pub struct StdProcessRunner {
    allowlist: ProcessAllowlist,
    timeout: Duration,
}

impl StdProcessRunner {
    pub fn new(allowlist: ProcessAllowlist) -> Self {
        Self {
            allowlist,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Stop a program that runs longer than `timeout`.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn allowlist(&self) -> &ProcessAllowlist {
        &self.allowlist
    }

    /// The root check again, after resolving symlinks.
    ///
    /// The allowlist compares paths as written, so `root/link` pointing elsewhere would
    /// pass it. Resolving both sides closes that — the same approach as the workspace
    /// filesystem adapter.
    async fn check_resolved_cwd(&self, cwd: &Path) -> Result<()> {
        let roots = self.allowlist.roots();
        if roots.is_empty() {
            return Ok(());
        }

        let resolved = tokio::fs::canonicalize(cwd).await.map_err(|error| {
            AppError::validation(format!(
                "cannot resolve working directory `{}`: {error}",
                cwd.display()
            ))
        })?;

        for root in roots {
            if tokio::fs::canonicalize(root.as_path())
                .await
                .is_ok_and(|root| resolved.starts_with(root))
            {
                return Ok(());
            }
        }

        Err(AppError::Permission(format!(
            "working directory `{}` resolves outside the allowed roots",
            cwd.display()
        )))
    }
}

#[async_trait]
impl ProcessRunner for StdProcessRunner {
    async fn run(&self, program: &str, args: &[String], cwd: &Path) -> Result<ProcessOutput> {
        // The gate. Anything the product did not configure stops here.
        self.allowlist.check_invocation(program, args, cwd)?;
        self.check_resolved_cwd(cwd).await?;

        let mut child = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // Whatever happens to this future, the program does not outlive it.
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| {
                AppError::ExternalService(format!("cannot run `{program}`: {error}"))
            })?;

        let stdout = child.stdout.take().expect("stdout is piped");
        let stderr = child.stderr.take().expect("stderr is piped");

        let finished = tokio::time::timeout(self.timeout, async {
            tokio::try_join!(read_capped(stdout), read_capped(stderr), child.wait())
        })
        .await;

        let (stdout, stderr, status) = match finished {
            Ok(result) => result.map_err(|error| {
                AppError::ExternalService(format!("cannot read the output of `{program}`: {error}"))
            })?,
            Err(_) => {
                // Explicit rather than left to `kill_on_drop`, so the program is gone by
                // the time the caller hears about it.
                let _ = child.kill().await;
                return Err(AppError::ExternalService(format!(
                    "`{program}` did not finish within {:?} and was stopped",
                    self.timeout
                )));
            }
        };

        Ok(ProcessOutput {
            status: status.code().unwrap_or(-1),
            stdout,
            stderr,
        })
    }
}

/// Read a stream to its end, keeping at most `MAX_OUTPUT_BYTES`.
///
/// Truncate rather than fail: a program that printed too much still ran, and the
/// caller usually only needs the exit code. The excess is read and dropped rather than
/// left in the pipe — a program blocked on a full pipe would never exit.
async fn read_capped(mut stream: impl AsyncRead + Unpin) -> std::io::Result<Vec<u8>> {
    let mut kept = Vec::new();
    let mut total = 0usize;
    let mut chunk = [0u8; 8192];

    loop {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        total += read;
        let room = MAX_OUTPUT_BYTES - kept.len();
        kept.extend_from_slice(&chunk[..read.min(room)]);
    }

    if total > kept.len() {
        tracing::warn!(
            captured = total,
            limit = MAX_OUTPUT_BYTES,
            "process output truncated"
        );
    }
    Ok(kept)
}

#[cfg(test)]
mod tests {
    use super::*;
    use origin_domain::ErrorKind;
    use origin_platform::WorkspaceRoot;
    use std::path::PathBuf;

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

    /// A fresh directory under the temp dir, unique to this test process.
    fn scratch_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("origin-process-std-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    fn confined_to(root: &Path) -> StdProcessRunner {
        let (program, _) = probe();
        StdProcessRunner::new(
            ProcessAllowlist::new([program])
                .within_roots([WorkspaceRoot::new(root.to_path_buf()).expect("absolute")]),
        )
    }

    #[tokio::test]
    async fn an_unlisted_program_never_reaches_the_operating_system() {
        let (program, _) = probe();
        let runner = StdProcessRunner::new(ProcessAllowlist::new([program]));

        let error = runner
            .run("definitely-not-allowed", &[], Path::new("."))
            .await
            .unwrap_err();

        assert_eq!(error.kind(), ErrorKind::Permission);
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

    #[cfg(unix)]
    #[tokio::test]
    async fn output_beyond_the_cap_is_dropped_while_reading() {
        let runner = StdProcessRunner::new(ProcessAllowlist::new(["head"]));
        let args = ["-c", "5000000", "/dev/zero"].map(str::to_owned);

        let output = runner
            .run("head", &args, Path::new("."))
            .await
            .expect("head must run");

        assert_eq!(output.status, 0);
        assert_eq!(output.stdout.len(), MAX_OUTPUT_BYTES);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_program_that_runs_too_long_is_stopped() {
        let runner = StdProcessRunner::new(ProcessAllowlist::new(["sleep"]))
            .with_timeout(Duration::from_millis(200));
        let started = std::time::Instant::now();

        let error = runner
            .run("sleep", &["30".to_owned()], Path::new("."))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("did not finish"), "got: {error}");
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[tokio::test]
    async fn a_working_directory_outside_the_roots_is_refused() {
        let (program, args) = probe();
        let root = scratch_dir("root");
        let runner = confined_to(&root);

        runner
            .run(program, &args, &root)
            .await
            .expect("the root itself is allowed");

        let error = runner
            .run(program, &args, &std::env::temp_dir())
            .await
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Permission);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_symlink_cannot_lead_the_working_directory_out_of_its_root() {
        let (program, args) = probe();
        let root = scratch_dir("symlink-root");
        let link = root.join("escape");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(std::env::temp_dir(), &link).expect("create symlink");

        // Lexically `root/escape` is inside the root; resolved, it is not.
        let error = confined_to(&root)
            .run(program, &args, &link)
            .await
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Permission);
    }
}
