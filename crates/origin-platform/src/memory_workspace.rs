//! Memory double for [`WorkspaceFs`] — test-only in-memory filesystem.
//!
//! Never use this in a shipped application. It exists so tests and contract suites
//! can validate behaviour without touching the real filesystem.

use crate::workspace::{DirEntry, RelPath, WorkspaceFs, WorkspaceRoot};
use async_trait::async_trait;
use origin_domain::{AppError, Result};
use std::collections::HashMap;
use std::sync::Mutex;

type FileMap = HashMap<(WorkspaceRoot, RelPath), Vec<u8>>;
type DirMap = HashMap<(WorkspaceRoot, RelPath), Vec<DirEntry>>;

/// In-memory filesystem whose contents are seeded by tests.
///
/// Every access is scoped to a [`WorkspaceRoot`] — a path under a different
/// root is a different set of entries.
#[derive(Debug, Default)]
pub struct MemoryWorkspaceFs {
    files: Mutex<FileMap>,
    dirs: Mutex<DirMap>,
}

impl MemoryWorkspaceFs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed a file so `read_file` returns its content.
    pub fn seed_file(&self, root: &WorkspaceRoot, path: &RelPath, content: Vec<u8>) {
        self.files
            .lock()
            .expect("poisoned")
            .insert((root.clone(), path.clone()), content);
    }

    /// Seed a directory listing — `list_dir` returns these entries.
    pub fn seed_dir(&self, root: &WorkspaceRoot, path: &RelPath, entries: Vec<DirEntry>) {
        self.dirs
            .lock()
            .expect("poisoned")
            .insert((root.clone(), path.clone()), entries);
    }
}

#[async_trait]
impl WorkspaceFs for MemoryWorkspaceFs {
    async fn list_dir(&self, root: &WorkspaceRoot, path: &RelPath) -> Result<Vec<DirEntry>> {
        let dirs = self.dirs.lock().expect("poisoned");
        Ok(dirs
            .get(&(root.clone(), path.clone()))
            .cloned()
            .unwrap_or_default())
    }

    async fn read_file(&self, root: &WorkspaceRoot, path: &RelPath) -> Result<Vec<u8>> {
        let files = self.files.lock().expect("poisoned");
        files
            .get(&(root.clone(), path.clone()))
            .cloned()
            .ok_or_else(|| {
                AppError::validation(format!(
                    "file not found: {}/{}",
                    root.as_path().display(),
                    path.as_path().display()
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn seeded_file_is_readable() {
        let fs = MemoryWorkspaceFs::new();
        let root = WorkspaceRoot::new(std::env::temp_dir()).unwrap();
        let path = RelPath::new("README.md").unwrap();

        fs.seed_file(&root, &path, b"hello world".to_vec());

        let content = fs.read_file(&root, &path).await.unwrap();
        assert_eq!(content, b"hello world");
    }

    #[tokio::test]
    async fn unseeded_file_returns_an_error() {
        let fs = MemoryWorkspaceFs::new();
        let root = WorkspaceRoot::new(std::env::temp_dir()).unwrap();

        let result = fs.read_file(&root, &RelPath::new("nope").unwrap()).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn seeded_dir_returns_entries() {
        let fs = MemoryWorkspaceFs::new();
        let root = WorkspaceRoot::new(std::env::temp_dir()).unwrap();
        let dir = RelPath::new("src").unwrap();

        fs.seed_dir(
            &root,
            &dir,
            vec![DirEntry::file("main.rs"), DirEntry::directory("lib")],
        );

        let entries = fs.list_dir(&root, &dir).await.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "main.rs");
        assert!(!entries[0].is_dir);
        assert!(entries[1].is_dir);
    }

    #[tokio::test]
    async fn different_roots_are_isolated() {
        let fs = MemoryWorkspaceFs::new();
        let root_a = WorkspaceRoot::new(std::env::temp_dir().join("a")).unwrap();
        let root_b = WorkspaceRoot::new(std::env::temp_dir().join("b")).unwrap();
        let path = RelPath::new("shared.txt").unwrap();

        fs.seed_file(&root_a, &path, b"from A".to_vec());
        fs.seed_file(&root_b, &path, b"from B".to_vec());

        let content_a = fs.read_file(&root_a, &path).await.unwrap();
        let content_b = fs.read_file(&root_b, &path).await.unwrap();

        assert_eq!(content_a, b"from A");
        assert_eq!(content_b, b"from B");
    }
}
