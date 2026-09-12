//! The workspace filesystem contract (`WorkspaceFs`) over the local filesystem (B2).
//!
//! Read-only by construction: the trait has no write method, so a product that uses
//! this adapter cannot modify a repository through it. Every access is confined to a
//! user-confirmed [`WorkspaceRoot`]; a path that escapes it — including via a symlink —
//! is refused with a permission error.

use async_trait::async_trait;
use origin_domain::{AppError, Result};
use origin_platform::{DirEntry, RelPath, WorkspaceFs, WorkspaceRoot};

/// Reads files and lists directories with the standard library.
///
/// No Tauri, no plugin: the OS filesystem is reachable from plain Rust, and the
/// allowlist/containment check in this adapter *is* the security boundary. ADR-0007
/// still applies to the frontend, which never gets `fs:*` — it goes through the
/// transport and the application services.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdWorkspaceFs;

impl StdWorkspaceFs {
    pub fn new() -> Self {
        Self
    }
}

/// Resolve `path` under `root`, refusing anything that escapes it.
///
/// Canonicalising both sides closes the symlink hole that a pure `..`-filter leaves
/// open: a symlink inside the root pointing outside it resolves to a path that no
/// longer starts with the canonicalised root.
fn resolve(root: &WorkspaceRoot, path: &RelPath) -> Result<std::path::PathBuf> {
    let canonical_root = std::fs::canonicalize(root.as_path()).map_err(|error| {
        AppError::storage(format!(
            "cannot resolve workspace root {}: {error}",
            root.as_path().display()
        ))
    })?;

    let joined = root.as_path().join(path.as_path());
    let canonical = std::fs::canonicalize(&joined).map_err(|error| {
        AppError::storage(format!("cannot resolve {}: {error}", joined.display()))
    })?;

    if !canonical.starts_with(&canonical_root) {
        return Err(AppError::Permission(format!(
            "{} escapes the workspace root",
            path.as_path().display()
        )));
    }

    Ok(canonical)
}

#[async_trait]
impl WorkspaceFs for StdWorkspaceFs {
    async fn list_dir(&self, root: &WorkspaceRoot, path: &RelPath) -> Result<Vec<DirEntry>> {
        let directory = resolve(root, path)?;

        let mut reader = tokio::fs::read_dir(&directory).await.map_err(|error| {
            AppError::storage(format!("cannot read {}: {error}", directory.display()))
        })?;

        let mut entries = Vec::new();
        while let Some(entry) = reader.next_entry().await.map_err(|error| {
            AppError::storage(format!("cannot read {}: {error}", directory.display()))
        })? {
            let file_type = entry.file_type().await.map_err(|error| {
                AppError::storage(format!("cannot stat {}: {error}", entry.path().display()))
            })?;

            entries.push(DirEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                is_dir: file_type.is_dir(),
            });
        }

        // Stable order: a directory listing that reshuffles between calls makes tests
        // flaky and diffs noisy.
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    async fn read_file(&self, root: &WorkspaceRoot, path: &RelPath) -> Result<Vec<u8>> {
        let file = resolve(root, path)?;

        tokio::fs::read(&file)
            .await
            .map_err(|error| AppError::storage(format!("cannot read {}: {error}", file.display())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_root(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("origin-workspace-fs-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn it_reads_and_lists_within_the_root() {
        let root_path = temp_root("read");
        std::fs::write(root_path.join("a.txt"), b"alpha").unwrap();
        std::fs::create_dir(root_path.join("sub")).unwrap();
        std::fs::write(root_path.join("sub").join("b.txt"), b"beta").unwrap();

        let root = WorkspaceRoot::new(root_path.clone()).unwrap();
        let fs = StdWorkspaceFs::new();

        let contents = fs
            .read_file(&root, &RelPath::new("a.txt").unwrap())
            .await
            .unwrap();
        assert_eq!(contents, b"alpha");

        let entries = fs
            .list_dir(&root, &RelPath::new(".").unwrap())
            .await
            .unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["a.txt", "sub"]);
        assert!(entries.iter().any(|e| e.name == "sub" && e.is_dir));

        std::fs::remove_dir_all(&root_path).ok();
    }

    /// Unix only: creating a symlink needs privileges on Windows.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_symlink_escaping_the_root_is_refused() {
        let root_path = temp_root("symlink-root");
        let outside = temp_root("symlink-outside");
        let secret = outside.join("secret.txt");
        std::fs::write(&secret, b"do not read").unwrap();

        // A file inside the root that is actually a symlink to the outside.
        std::os::unix::fs::symlink(&secret, root_path.join("escape.txt")).unwrap();

        let root = WorkspaceRoot::new(root_path.clone()).unwrap();
        let fs = StdWorkspaceFs::new();

        let error = fs
            .read_file(&root, &RelPath::new("escape.txt").unwrap())
            .await
            .unwrap_err();

        assert_eq!(
            error.kind(),
            origin_domain::ErrorKind::Permission,
            "a symlink out of the root must be a permission error, got: {error}"
        );

        std::fs::remove_dir_all(&root_path).ok();
        std::fs::remove_dir_all(&outside).ok();
    }
}
