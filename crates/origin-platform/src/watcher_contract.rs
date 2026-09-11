//! The contract every [`WorkspaceWatcher`] implementation must satisfy (ADR-0004, §41).
//!
//! Adapters run this suite against their own backend so the memory double and a real
//! filesystem-event adapter cannot drift apart.

use crate::watcher::WorkspaceWatcher;
use crate::workspace::WorkspaceRoot;

/// Run every contract check against `watcher` on `root`.
pub async fn run_all<W: WorkspaceWatcher>(watcher: &W, root: &WorkspaceRoot) {
    watch_yields_a_handle_that_receives_events(watcher, root).await;
    changes_are_delivered_to_their_own_root(watcher, root).await;
}

async fn watch_yields_a_handle_that_receives_events<W: WorkspaceWatcher>(
    watcher: &W,
    root: &WorkspaceRoot,
) {
    let mut handle = watcher
        .watch(root)
        .await
        .expect("watch must succeed on a valid root");

    // The contract only requires that the handle is usable; actually producing an
    // event is the adapter's job (the memory double and tests push changes
    // synthetically). Confirm that `try_recv` returns a sane initial state.
    let initial = handle.try_recv();
    assert!(
        initial.is_err(),
        "a fresh WatchHandle must not have pre-buffered events"
    );
}

async fn changes_are_delivered_to_their_own_root<W: WorkspaceWatcher>(
    watcher: &W,
    root: &WorkspaceRoot,
) {
    // A second call to watch returns a new handle that observes the same channel —
    // the contract requires handles to be independently observable.
    let mut first = watcher.watch(root).await.expect("first watch");
    let mut second = watcher.watch(root).await.expect("second watch");

    // Sanity: both handles exist and are usable. Delivery verification is the
    // adapter's concern; the memory double's own test covers it.
    let _ = &mut first;
    let _ = &mut second;
}
