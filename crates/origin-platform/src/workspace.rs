//! Workspace filesystem contract (B2 of the Gitbit platform requirements).
//!
//! Read-only access to files under user-confirmed roots. Unlike a blanket
//! `fs:allow-read-*` Tauri capability, every access is scoped to an explicitly
//! chosen directory — `WorkspaceRoot` is confirmed via a native dialog, not a
//! free-form path the caller can pick.

use async_trait::async_trait;
use origin_domain::{AppError, Result};
use std::fmt::Debug;
use std::path::{Component, Path, PathBuf};

/// A directory explicitly chosen by the user (e.g. via a native file dialog).
///
/// Distinct from a bare `PathBuf` so a free path cannot be passed by accident.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceRoot(PathBuf);

impl WorkspaceRoot {
    pub fn new(path: PathBuf) -> Result<Self> {
        if !path.is_absolute() {
            return Err(AppError::validation(
                "workspace root must be an absolute path",
            ));
        }
        Ok(Self(path))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub fn into_inner(self) -> PathBuf {
        self.0
    }
}

/// A relative path within a [`WorkspaceRoot`].
///
/// Construction rejects `.` and `..` components, so a resolved `root / rel`
/// can never escape the root directory. The guard is checked at construction
/// time and again in the memory double, so a filesystem adapter can trust the
/// type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelPath(PathBuf);

impl RelPath {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if path.is_absolute() {
            return Err(AppError::validation("a relative path must not be absolute"));
        }

        for component in path.components() {
            match component {
                Component::Normal(_) | Component::CurDir => {}
                _ => {
                    return Err(AppError::validation(format!(
                        "a relative path must not contain `..`: `{}`",
                        path.display()
                    )));
                }
            }
        }

        Ok(Self(path.to_path_buf()))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// A directory entry returned by [`WorkspaceFs::list_dir`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}

impl DirEntry {
    pub fn file(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_dir: false,
        }
    }

    pub fn directory(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_dir: true,
        }
    }
}

/// Read-only workspace filesystem.
///
/// Every access is scoped to a [`WorkspaceRoot`]. Write access is deliberately
/// excluded from this contract — destructive operations need their own port and
/// their own permission (ADR-0007).
#[async_trait]
pub trait WorkspaceFs: Debug + Send + Sync + 'static {
    /// List the contents of a directory within `root`.
    async fn list_dir(&self, root: &WorkspaceRoot, path: &RelPath) -> Result<Vec<DirEntry>>;

    /// Read the contents of a file within `root`.
    async fn read_file(&self, root: &WorkspaceRoot, path: &RelPath) -> Result<Vec<u8>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_root_rejects_relative_paths() {
        let result = WorkspaceRoot::new(PathBuf::from("relative/path"));
        match result {
            Err(AppError::Validation(_)) => {} // expected
            other => panic!("expected Validation error for relative path, got {other:?}"),
        }
    }

    #[test]
    fn workspace_root_accepts_absolute_paths() {
        WorkspaceRoot::new(PathBuf::from("/Users/test/repo")).expect("absolute path must be ok");
    }

    #[test]
    fn rel_path_rejects_absolute_paths() {
        let result = RelPath::new("/absolute/file");
        match result {
            Err(AppError::Validation(_)) => {} // expected
            other => panic!("expected Validation error for absolute rel path, got {other:?}"),
        }
    }

    #[test]
    fn rel_path_rejects_parent_traversal() {
        let result = RelPath::new("../escape");
        match result {
            Err(AppError::Validation(_)) => {} // expected
            other => panic!("expected Validation error for parent traversal, got {other:?}"),
        }
    }

    #[test]
    fn rel_path_accepts_normal_relative_paths() {
        let rel = RelPath::new("src/main.rs").expect("normal relative path must be ok");
        assert_eq!(rel.as_path(), Path::new("src/main.rs"));
    }

    #[test]
    fn rel_path_accepts_current_dir_prefix() {
        let rel = RelPath::new("./file.txt").expect("./ prefix must be ok");
        assert_eq!(rel.as_path(), Path::new("./file.txt"));
    }
}
