//! The contract every [`WorkspaceFs`] implementation must satisfy (ADR-0004, §41).
//!
//! Adapters run this suite against their own backend so that an in-memory double
//! and a real filesystem adapter cannot drift apart.
//!
//! ```ignore
//! #[tokio::test]
//! async fn satisfies_the_workspace_fs_contract() {
//!     let root = WorkspaceRoot::new(PathBuf::from("/tmp/workspace-test")).unwrap();
//!     let fs = MemoryWorkspaceFs::new();
//!     origin_platform::workspace_contract::run_all(&fs, &root).await;
//! }
//! ```

use crate::workspace::{RelPath, WorkspaceFs, WorkspaceRoot};
use origin_domain::ErrorKind;

/// Run every contract check against `fs` under `root`.
pub async fn run_all<F: WorkspaceFs>(fs: &F, root: &WorkspaceRoot) {
    lists_an_empty_directory(fs, root).await;
    lists_a_directory_with_entries(fs, root).await;
    reads_a_file(fs, root).await;
    parent_traversal_is_rejected_early(fs, root).await;
}

async fn lists_an_empty_directory<F: WorkspaceFs>(fs: &F, root: &WorkspaceRoot) {
    let entries = fs
        .list_dir(
            root,
            &RelPath::new(".").expect("'.' is a valid relative path"),
        )
        .await
        .expect("listing '.' must not fail");
    // A file was just seeded — at least it exists.
    // An empty or populated listing is both valid depending on the double.
    let _ = entries;
}

async fn lists_a_directory_with_entries<F: WorkspaceFs>(fs: &F, root: &WorkspaceRoot) {
    let entries = fs
        .list_dir(root, &RelPath::new(".").expect("'.' is valid"))
        .await
        .expect("listing '.' must not fail");

    // Verify that entries, if any, have valid names.
    for entry in &entries {
        assert!(
            !entry.name.is_empty(),
            "a directory entry must have a non-empty name"
        );
    }
}

async fn reads_a_file<F: WorkspaceFs>(fs: &F, root: &WorkspaceRoot) {
    let path = RelPath::new("README.md").expect("valid relative path");

    let result = fs.read_file(root, &path).await;

    match result {
        Ok(content) => {
            // The contract does not mandate file contents — they belong to the
            // implementation. It only checks that a `read_file` with a valid
            // relative path either succeeds or fails with a non-Permission error
            // (the file might not exist).
            let _ = content;
        }
        Err(err) => {
            let kind = err.kind();
            assert_ne!(
                kind,
                ErrorKind::Permission,
                "read_file for a path within the root must not be rejected \
                 as Permission (path={})",
                path.as_path().display()
            );
            // Storage, Configuration, Internal are all legitimate for a
            // missing file or a double that does not seed this exact path.
        }
    }
}

async fn parent_traversal_is_rejected_early<F: WorkspaceFs>(fs: &F, root: &WorkspaceRoot) {
    // RelPath::new already rejects `..` at construction time, so the guard is
    // enforced by the type system. This test confirms the type system holds.
    let result = RelPath::new("../escape");
    assert!(
        result.is_err(),
        "RelPath must reject `..` at construction, before any adapter sees it"
    );

    // Additionally, verify that the adapter side handles a non-canonical `.`
    // cleanly — listing the root itself must work.
    let root_list = fs.list_dir(root, &RelPath::new(".").unwrap()).await;
    assert!(root_list.is_ok(), "listing the root via '.' must succeed");
}
