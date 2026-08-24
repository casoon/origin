//! Migrations run against frozen fixture projects.
//!
//! Without these, migrations are untested code that runs exactly once — in someone
//! else's repository (ADR-0025).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Copy a fixture into a scratch directory, so a test never mutates the checked-in one.
fn scratch(fixture: &str) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(fixture);

    // Unique per call: tests run in parallel in one process, so a name derived only
    // from the process id would have them clobbering each other's copy.
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let target = std::env::temp_dir().join(format!(
        "origin-fixture-{fixture}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&target);
    copy_tree(&source, &target);
    target
}

fn copy_tree(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).expect("create scratch directory");

    for entry in std::fs::read_dir(source).expect("read fixture").flatten() {
        let from = entry.path();
        let to = target.join(entry.file_name());

        if from.is_dir() {
            copy_tree(&from, &to);
        } else {
            std::fs::copy(&from, &to).expect("copy fixture file");
        }
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn a_convertible_capability_becomes_a_generated_one() {
    let project = scratch("converted");

    origin_xtask::update(&project, false).expect("update");

    assert!(
        !project.join("src-tauri/capabilities/default.json").exists(),
        "the hand-written file must be gone"
    );

    let generated = read(&project.join("src-tauri/capabilities/standard-dashboard.json"));
    assert!(
        generated.contains("Generated from app.toml"),
        "a generated file must take its place, got: {generated}"
    );
    assert!(generated.contains("core:event:allow-listen"));
}

#[test]
fn the_project_records_the_version_it_now_tracks() {
    let project = scratch("converted");

    origin_xtask::update(&project, false).expect("update");

    let manifest = read(&project.join("app.toml"));
    assert!(manifest.contains(r#"version = "0.2.0""#), "got: {manifest}");
    assert!(
        manifest.contains("# A project on origin 0.1.0"),
        "comments must survive — this is a file a human wrote and will read again"
    );
}

#[test]
fn running_twice_changes_nothing_the_second_time() {
    let project = scratch("converted");

    origin_xtask::update(&project, false).expect("first update");
    let after_first = read(&project.join("src-tauri/capabilities/standard-dashboard.json"));

    origin_xtask::update(&project, false).expect("second update");
    let after_second = read(&project.join("src-tauri/capabilities/standard-dashboard.json"));

    assert_eq!(after_first, after_second, "migrations must be idempotent");
}

#[test]
fn a_capability_no_profile_covers_is_handed_back_to_a_human() {
    let project = scratch("legacy");
    let capability = project.join("src-tauri/capabilities/default.json");

    origin_xtask::update(&project, false).expect("update");

    assert!(
        capability.exists(),
        "a migration must not silently drop permissions it cannot map — the file stays \
         until someone decides"
    );
    assert!(
        read(&capability).contains("fs:allow-write-text-file"),
        "and it stays untouched"
    );
}

#[test]
fn a_dry_run_writes_nothing() {
    let project = scratch("converted");
    let before = read(&project.join("app.toml"));

    origin_xtask::update(&project, true).expect("dry run");

    assert!(project.join("src-tauri/capabilities/default.json").exists());
    assert_eq!(read(&project.join("app.toml")), before);
}

#[test]
fn a_project_from_the_future_is_refused_rather_than_downgraded() {
    let project = scratch("converted");
    let manifest = project.join("app.toml");
    std::fs::write(
        &manifest,
        read(&manifest).replace(r#"version = "0.1.0""#, r#"version = "9.0.0""#),
    )
    .expect("write manifest");

    let error = origin_xtask::update(&project, false).unwrap_err();

    assert!(error.contains("only knows up to"), "got: {error}");
}
