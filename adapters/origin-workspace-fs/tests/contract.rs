//! The shared `WorkspaceFs` contract, run against the real filesystem.

use origin_platform::WorkspaceRoot;
use origin_workspace_fs::StdWorkspaceFs;

#[tokio::test]
async fn std_workspace_fs_satisfies_the_workspace_fs_contract() {
    let dir = std::env::temp_dir().join(format!(
        "origin-workspace-fs-contract-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("README.md"), b"# Origin").unwrap();

    let root = WorkspaceRoot::new(dir.clone()).expect("absolute root");
    let fs = StdWorkspaceFs::new();

    // The trait must be in scope for the contract's method calls to resolve.
    origin_platform::workspace_contract::run_all(&fs, &root).await;

    std::fs::remove_dir_all(&dir).ok();
}
