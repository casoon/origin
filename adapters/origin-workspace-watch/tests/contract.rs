//! The shared `WorkspaceWatcher` contract, run against `notify`.

use origin_platform::WorkspaceRoot;
use origin_workspace_watch::NotifyWorkspaceWatcher;

#[tokio::test]
async fn notify_watcher_satisfies_the_watcher_contract() {
    let dir = std::env::temp_dir().join(format!(
        "origin-workspace-watch-contract-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let root = WorkspaceRoot::new(dir.clone()).expect("absolute root");
    let watcher = NotifyWorkspaceWatcher::new();

    origin_platform::watcher_contract::run_all(&watcher, &root).await;

    std::fs::remove_dir_all(&dir).ok();
}
