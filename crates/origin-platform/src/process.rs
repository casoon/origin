//! Process execution contract (B1 of the Gitbit platform requirements).
//!
//! Anything that runs local programs needs its own contract and its own capability
//! (ADR-0007). This is deliberately not a general shell escape: the allowlist is
//! configuration — a product lists its git, its editors, its terminals.

use crate::workspace::WorkspaceRoot;
use async_trait::async_trait;
use origin_domain::{AppError, Result};
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::{Component, Path};

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

/// The programs a [`ProcessRunner`] may start, and with what.
///
/// Configuration, never code: a product declares its allowed programs (git,
/// its configured editors, its terminal launchers). Every implementation must
/// call [`ProcessAllowlist::check_invocation`] before handing a program to the
/// operating system — the contract test enforces this.
#[derive(Debug, Clone, Default)]
pub struct ProcessAllowlist {
    programs: Vec<String>,
    /// Arguments refused per program, although the program itself is allowed.
    denied_arguments: BTreeMap<String, Vec<String>>,
    /// When non-empty, the working directory must lie under one of these.
    roots: Vec<WorkspaceRoot>,
}

impl ProcessAllowlist {
    pub fn new(programs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            programs: programs.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    /// Refuse `arguments` for `program`, although the program itself is allowed.
    ///
    /// Allowing a program allows everything its arguments can make it do, and some
    /// arguments turn a harmless tool into a command runner — `git -c
    /// core.sshCommand=…`, `git fetch --upload-pack=…`. Listing them closes those
    /// paths:
    ///
    /// ```
    /// # use origin_platform::ProcessAllowlist;
    /// let allowlist = ProcessAllowlist::new(["git"]).deny_arguments(
    ///     "git",
    ///     ["-c", "--config-env", "--exec-path", "--upload-pack", "--receive-pack"],
    /// );
    /// ```
    ///
    /// An entry matches the argument itself, its `--flag=value` form, and for a
    /// short flag its attached form (`-cvalue`). It is a deny list, so it covers only
    /// what it names: a module should still never pass untrusted input as arguments.
    pub fn deny_arguments(
        mut self,
        program: impl Into<String>,
        arguments: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.denied_arguments
            .entry(program.into())
            .or_default()
            .extend(arguments.into_iter().map(Into::into));
        self
    }

    /// Only accept working directories under one of `roots`.
    ///
    /// Compared by path components, without touching the filesystem; a runner that
    /// can resolve symlinks checks the resolved path as well (`StdProcessRunner` does).
    pub fn within_roots(mut self, roots: impl IntoIterator<Item = WorkspaceRoot>) -> Self {
        self.roots.extend(roots);
        self
    }

    pub fn allows(&self, program: &str) -> bool {
        self.programs.iter().any(|p| p == program)
    }

    /// Reject a program not in the allowlist.
    pub fn check(&self, program: &str) -> Result<()> {
        if self.allows(program) {
            Ok(())
        } else {
            Err(AppError::Permission(format!(
                "program `{program}` is not in the process allowlist"
            )))
        }
    }

    /// The full gate: program, arguments and working directory.
    ///
    /// The shared gate every implementation must call, so an allowlist violation
    /// can never reach the operating system. The contract test verifies that
    /// implementations actually delegate to this check.
    pub fn check_invocation(&self, program: &str, args: &[String], cwd: &Path) -> Result<()> {
        self.check(program)?;

        let denied = self.denied_arguments(program);
        if let Some(argument) = args
            .iter()
            .find(|argument| denied.iter().any(|flag| matches_flag(argument, flag)))
        {
            return Err(AppError::Permission(format!(
                "argument `{argument}` is not allowed for `{program}`"
            )));
        }

        if !self.roots.is_empty() && !self.roots.iter().any(|root| is_within(cwd, root)) {
            return Err(AppError::Permission(format!(
                "working directory `{}` is outside the allowed roots",
                cwd.display()
            )));
        }

        Ok(())
    }

    /// The programs currently allowed.
    pub fn entries(&self) -> &[String] {
        &self.programs
    }

    /// The arguments refused for `program`.
    pub fn denied_arguments(&self, program: &str) -> &[String] {
        self.denied_arguments
            .get(program)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// The roots working directories must lie under; empty means any.
    pub fn roots(&self) -> &[WorkspaceRoot] {
        &self.roots
    }
}

/// `--flag`, `--flag=value`, and for a short flag also `-fvalue`.
fn matches_flag(argument: &str, flag: &str) -> bool {
    let Some(rest) = argument.strip_prefix(flag) else {
        return false;
    };
    let is_short = flag.len() == 2 && flag.starts_with('-') && !flag.starts_with("--");
    rest.is_empty() || rest.starts_with('=') || is_short
}

/// Lexical containment: `path` is `root` or below it, with no `..` to climb back out.
fn is_within(path: &Path, root: &WorkspaceRoot) -> bool {
    path.is_absolute()
        && path.starts_with(root.as_path())
        && !path
            .components()
            .any(|component| component == Component::ParentDir)
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
    use origin_domain::ErrorKind;
    use std::path::PathBuf;

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

    fn check_git(allowlist: &ProcessAllowlist, args: &[&str]) -> Result<()> {
        let args: Vec<String> = args.iter().map(|argument| (*argument).to_owned()).collect();
        allowlist.check_invocation("git", &args, &std::env::temp_dir())
    }

    fn git_without_command_execution() -> ProcessAllowlist {
        ProcessAllowlist::new(["git", "code"]).deny_arguments("git", ["-c", "--upload-pack"])
    }

    #[test]
    fn a_denied_argument_is_refused_in_every_spelling() {
        let allowlist = git_without_command_execution();
        let cases: [&[&str]; 4] = [
            &["-c", "core.sshCommand=touch /tmp/pwned", "fetch"],
            &["-ccore.sshCommand=touch /tmp/pwned", "fetch"],
            &["fetch", "--upload-pack=touch /tmp/pwned"],
            &["fetch", "--upload-pack", "touch /tmp/pwned"],
        ];

        for args in cases {
            let error = check_git(&allowlist, args).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Permission, "{args:?}");
        }
    }

    #[test]
    fn arguments_that_only_look_alike_stay_allowed() {
        let allowlist = git_without_command_execution();

        check_git(&allowlist, &["diff", "--cached"]).expect("--cached is not -c");
        check_git(&allowlist, &["-C", "repo", "status"]).expect("-C is not -c");
        check_git(&allowlist, &["fetch", "--upload-packs"]).expect("not the same long flag");
    }

    #[test]
    fn a_rule_for_one_program_does_not_restrict_another() {
        let allowlist = git_without_command_execution();

        allowlist
            .check_invocation("code", &["-c".to_owned()], &std::env::temp_dir())
            .expect("the rule is for git only");
    }

    #[test]
    fn the_working_directory_must_lie_under_a_root() {
        let root = std::env::temp_dir().join("origin-process-root");
        let allowlist = ProcessAllowlist::new(["git"])
            .within_roots([WorkspaceRoot::new(root.clone()).expect("absolute")]);

        allowlist
            .check_invocation("git", &[], &root)
            .expect("the root itself");
        allowlist
            .check_invocation("git", &[], &root.join("repo"))
            .expect("below the root");

        let outside = [
            std::env::temp_dir().join("elsewhere"),
            root.join("..").join("elsewhere"),
            PathBuf::from("relative"),
        ];
        for cwd in outside {
            let error = allowlist.check_invocation("git", &[], &cwd).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Permission, "{}", cwd.display());
        }
    }

    #[test]
    fn without_roots_any_working_directory_is_accepted() {
        ProcessAllowlist::new(["git"])
            .check_invocation("git", &[], Path::new("."))
            .expect("no roots configured");
    }
}
