//! Memory double for [`ProcessRunner`] — records calls and returns configured outputs.
//!
//! Never use this in a shipped application. It exists so tests and contract suites
//! can validate behaviour without starting real processes.

use crate::process::{ProcessAllowlist, ProcessOutput, ProcessRunner};
use async_trait::async_trait;
use origin_domain::Result;
use std::path::Path;
use std::sync::Mutex;

/// Records process invocations and returns pre-configured outputs.
///
/// Programs not in the allowlist are rejected with `AppError::Permission`
/// before the `run` method returns — exactly as the contract requires.
#[derive(Debug)]
pub struct MemoryProcessRunner {
    allowlist: ProcessAllowlist,
    output: ProcessOutput,
    /// (program, args, cwd) for every run that passed the allowlist gate.
    calls: Mutex<Vec<(String, Vec<String>, String)>>,
}

impl MemoryProcessRunner {
    pub fn new(allowlist: ProcessAllowlist, output: ProcessOutput) -> Self {
        Self {
            allowlist,
            output,
            calls: Mutex::new(Vec::new()),
        }
    }

    /// Construct a runner that returns a successful exit code (0) and empty output.
    pub fn success(allowlist: ProcessAllowlist) -> Self {
        Self::new(
            allowlist,
            ProcessOutput {
                status: 0,
                stdout: Vec::new(),
                stderr: Vec::new(),
            },
        )
    }

    /// Every call that passed the allowlist gate.
    pub fn calls(&self) -> Vec<(String, Vec<String>, String)> {
        self.calls.lock().expect("recorder poisoned").clone()
    }
}

#[async_trait]
impl ProcessRunner for MemoryProcessRunner {
    async fn run(&self, program: &str, args: &[String], cwd: &Path) -> Result<ProcessOutput> {
        self.allowlist.check(program)?;

        self.calls.lock().expect("recorder poisoned").push((
            program.to_owned(),
            args.to_vec(),
            cwd.display().to_string(),
        ));

        Ok(self.output.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_unlisted_programs() {
        let allowlist = ProcessAllowlist::new(["git"]);
        let runner = MemoryProcessRunner::new(
            allowlist,
            ProcessOutput {
                status: 0,
                stdout: vec![],
                stderr: vec![],
            },
        );

        let result = runner.run("rm", &[], Path::new("/")).await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind(),
            origin_domain::ErrorKind::Permission
        );
    }

    #[tokio::test]
    async fn allows_listed_programs_and_records_calls() {
        let allowlist = ProcessAllowlist::new(["git", "npm"]);
        let output = ProcessOutput {
            status: 0,
            stdout: b"ok".to_vec(),
            stderr: vec![],
        };
        let runner = MemoryProcessRunner::new(allowlist, output.clone());

        let result = runner
            .run("git", &["status".to_owned()], Path::new("/repo"))
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), output);

        let calls = runner.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "git");
        assert_eq!(calls[0].1, vec!["status"]);
        assert!(calls[0].2.contains("repo"));
    }
}
