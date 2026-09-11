//! Integration tests: verify that the memory doubles satisfy the shared contract suites.

use origin_platform::{
    MemoryProcessRunner, MemoryWorkspaceFs, MemoryWorkspaceWatcher, ProcessAllowlist,
    ProcessOutput, RelPath, WorkspaceRoot,
};
use std::path::Path;

// ── Process contract ─────────────────────────────────────────────────────

#[tokio::test]
async fn memory_process_runner_satisfies_the_process_contract() {
    let allowed = "contract-test-git";
    let allowlist = ProcessAllowlist::new([allowed]);
    let runner = MemoryProcessRunner::new(
        allowlist.clone(),
        ProcessOutput {
            status: 0,
            stdout: b"contract output".to_vec(),
            stderr: vec![],
        },
    );

    origin_platform::process_contract::run_all(&runner, &allowlist).await;
}

// ── Workspace filesystem contract ────────────────────────────────────────

#[tokio::test]
async fn memory_workspace_fs_satisfies_the_workspace_fs_contract() {
    let root = WorkspaceRoot::new(Path::new("/tmp/origin-workspace-contract-test").to_path_buf())
        .expect("absolute root");
    let fs = MemoryWorkspaceFs::new();

    // Seed some data so the contract's read/ls assertions find something.
    fs.seed_file(
        &root,
        &RelPath::new("README.md").unwrap(),
        b"# Origin".to_vec(),
    );
    fs.seed_dir(
        &root,
        &RelPath::new(".").unwrap(),
        vec![
            origin_platform::DirEntry::file("README.md"),
            origin_platform::DirEntry::directory("src"),
        ],
    );

    origin_platform::workspace_contract::run_all(&fs, &root).await;
}

// ── Workspace watcher contract ───────────────────────────────────────────

#[tokio::test]
async fn memory_workspace_watcher_satisfies_the_watcher_contract() {
    let root = WorkspaceRoot::new(Path::new("/tmp/origin-watcher-contract-test").to_path_buf())
        .expect("absolute root");
    let watcher = MemoryWorkspaceWatcher::new();

    origin_platform::watcher_contract::run_all(&watcher, &root).await;
}
