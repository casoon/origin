//! Process execution contract (B1 of the Gitbit platform requirements).
//!
//! Anything that runs local programs needs its own contract and its own capability
//! (ADR-0007). This is deliberately not a general shell escape: the allowlist is
//! configuration — a product lists its git, its editors, its terminals.

use async_trait::async_trait;
use origin_domain::{AppError, Result};
use std::fmt::Debug;
use std::path::Path;

/// The result of running a process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl ProcessOutput {
    pub fn success(&self) -> bool {
        self.status == 0
    }
}

/// The programs a [`ProcessRunner`] may start.
///
/// Configuration, never code: a product declares its allowed programs (git,
/// its configured editors, its terminal launchers). Every implementation must
/// call [`ProcessAllowlist::check`] before handing a program to the operating
/// system — the contract test enforces this.
#[derive(Debug, Clone, Default)]
pub struct ProcessAllowlist {
    programs: Vec<String>,
}

impl ProcessAllowlist {
    pub fn new(programs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            programs: programs.into_iter().map(Into::into).collect(),
        }
    }

    pub fn allows(&self, program: &str) -> bool {
        self.programs.iter().any(|p| p == program)
    }

    /// Reject a program not in the allowlist.
    ///
    /// The shared gate every implementation must call, so an allowlist violation
    /// can never reach the operating system. The contract test verifies that
    /// implementations actually delegate to this check.
    pub fn check(&self, program: &str) -> Result<()> {
        if self.allows(program) {
            Ok(())
        } else {
            Err(AppError::Permission(format!(
                "program `{program}` is not in the process allowlist"
            )))
        }
    }

    /// The programs currently allowed.
    pub fn entries(&self) -> &[String] {
        &self.programs
    }
}

/// Runs a local program under a strict allowlist.
///
/// A product that does not need to start external programs never instantiates
/// this dependency — the contract is optional by construction.
#[async_trait]
pub trait ProcessRunner: Debug + Send + Sync + 'static {
    /// `program` must be in the configured allowlist.
    /// `cwd` is the working directory for the launched process.
    async fn run(&self, program: &str, args: &[String], cwd: &Path) -> Result<ProcessOutput>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_rejects_an_unlisted_program_with_permission_error() {
        let allowlist = ProcessAllowlist::new(["git"]);
        let result = allowlist.check("rm");

        match result {
            Err(AppError::Permission(_)) => {} // expected
            other => panic!("expected Permission error, got {other:?}"),
        }
    }

    #[test]
    fn allowlist_permits_a_listed_program() {
        let allowlist = ProcessAllowlist::new(["git", "code"]);
        assert!(allowlist.allows("git"));
        assert!(allowlist.allows("code"));
        allowlist
            .check("git")
            .expect("listed program must be allowed");
    }
}
