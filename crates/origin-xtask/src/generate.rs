//! Files derived from `app.toml` (ADR-0021, ADR-0022).
//!
//! Generated files are Origin-owned: they are overwritten on every run and must not be
//! edited. That is what makes an Origin upgrade a regeneration rather than a merge.

use crate::contracts;
use origin_manifest::{Capability, Manifest};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Marker written into every generated file, and the thing that tells the generator
/// which files it is allowed to remove.
const MARKER: &str = "Generated from app.toml";

/// Write everything derived from the manifests found in the workspace.
pub fn run(root: &Path) -> Result<(), String> {
    // Contracts are generated only where `@origin/client` itself lives — that is the
    // Origin repository. A derivative consumes the package and gets the bindings with
    // it, rather than regenerating a copy that could differ.
    if let Some(contracts) = contracts_target(root) {
        write_if_changed(&contracts, &contracts::render()?)?;
        println!("generated {}", relative(root, &contracts));
    }

    let manifests = find_manifests(root)?;
    if manifests.is_empty() {
        return Err(format!("no app.toml found under {}", root.display()));
    }

    for path in manifests {
        let manifest = load(&path)?;
        let directory = tauri_directory(&path);

        for (file, contents) in outputs(&manifest, &directory)? {
            write_if_changed(&file, &contents)?;
        }

        remove_stale_capabilities(&manifest, &directory)?;
        println!("generated files for {}", relative(root, &path));
    }

    Ok(())
}

/// Fail if anything generated is missing, stale, or was edited by hand.
///
/// Run in CI: a hand-edited generated file survives review far too easily, and a stale
/// one means the manifest and the build disagree.
pub fn check(root: &Path) -> Result<(), String> {
    let manifests = find_manifests(root)?;
    if manifests.is_empty() {
        return Err(format!("no app.toml found under {}", root.display()));
    }

    let mut problems = Vec::new();

    check_contracts(root, &mut problems)?;

    for path in manifests {
        let manifest = load(&path)?;
        let directory = tauri_directory(&path);

        check_manifest_outputs(root, &manifest, &directory, &mut problems)?;
        check_stale_capabilities(root, &manifest, &directory, &mut problems)?;
    }

    if problems.is_empty() {
        println!("generated files: up to date");
        return Ok(());
    }

    let mut report = String::from("generated files are out of date:\n");
    for problem in &problems {
        let _ = writeln!(report, "  - {problem}");
    }
    Err(report)
}

/// Compare the generated contracts bindings against what the Rust definitions would
/// produce, appending a problem if they differ or are missing.
fn check_contracts(root: &Path, problems: &mut Vec<String>) -> Result<(), String> {
    let Some(contracts_path) = contracts_target(root) else {
        return Ok(());
    };

    match std::fs::read_to_string(&contracts_path) {
        Ok(actual) if actual == contracts::render()? => {}
        Ok(_) => problems.push(format!(
            "{} no longer matches the Rust contracts — run `cargo xtask generate`",
            relative(root, &contracts_path)
        )),
        Err(_) => problems.push(format!(
            "{} is missing — run `cargo xtask generate`",
            relative(root, &contracts_path)
        )),
    }

    Ok(())
}

/// Compare one manifest's generated outputs against what is on disk, appending a
/// problem for each file that differs or is missing.
fn check_manifest_outputs(
    root: &Path,
    manifest: &Manifest,
    directory: &Path,
    problems: &mut Vec<String>,
) -> Result<(), String> {
    for (file, expected) in outputs(manifest, directory)? {
        match std::fs::read_to_string(&file) {
            Ok(actual) if actual == expected => {}
            Ok(_) => problems.push(format!(
                "{} differs from what app.toml describes — run `cargo xtask generate`",
                relative(root, &file)
            )),
            Err(_) => problems.push(format!(
                "{} is missing — run `cargo xtask generate`",
                relative(root, &file)
            )),
        }
    }

    Ok(())
}

/// Append a problem for each capability file this generator produced earlier but no
/// longer would.
fn check_stale_capabilities(
    root: &Path,
    manifest: &Manifest,
    directory: &Path,
    problems: &mut Vec<String>,
) -> Result<(), String> {
    for stale in stale_capabilities(manifest, directory)? {
        problems.push(format!(
            "{} is generated but no window uses its profile — run `cargo xtask generate`",
            relative(root, &stale)
        ));
    }

    Ok(())
}

/// Where generated bindings belong, if this workspace holds `@origin/client`.
fn contracts_target(root: &Path) -> Option<PathBuf> {
    let path = contracts::output_path(root);
    path.parent().is_some_and(Path::is_dir).then_some(path)
}

/// Everything one manifest produces, as `(path, contents)`.
fn outputs(manifest: &Manifest, tauri_directory: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    let mut outputs = Vec::new();
    let capabilities = tauri_directory.join("capabilities");

    for capability in Capability::from_manifest(manifest) {
        let json = serde_json::to_string_pretty(&capability)
            .map_err(|error| format!("cannot encode capability: {error}"))?;

        outputs.push((
            capabilities.join(capability.file_name()),
            format!("{json}\n"),
        ));
    }

    Ok(outputs)
}

/// Capability files this generator wrote earlier but no longer produces.
fn stale_capabilities(manifest: &Manifest, tauri_directory: &Path) -> Result<Vec<PathBuf>, String> {
    let capabilities = tauri_directory.join("capabilities");
    if !capabilities.exists() {
        return Ok(Vec::new());
    }

    let current: Vec<String> = Capability::from_manifest(manifest)
        .iter()
        .map(Capability::file_name)
        .collect();

    let mut stale = Vec::new();
    let entries = std::fs::read_dir(&capabilities)
        .map_err(|error| format!("cannot read {}: {error}", capabilities.display()))?;

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        if path.extension().is_none_or(|extension| extension != "json") || current.contains(&name) {
            continue;
        }

        // Only files this generator recognises as its own are ever removed. A
        // hand-written capability keeps working until someone converts it.
        let contents = std::fs::read_to_string(&path).unwrap_or_default();
        if contents.contains(MARKER) {
            stale.push(path);
        }
    }

    Ok(stale)
}

fn remove_stale_capabilities(manifest: &Manifest, tauri_directory: &Path) -> Result<(), String> {
    for path in stale_capabilities(manifest, tauri_directory)? {
        std::fs::remove_file(&path)
            .map_err(|error| format!("cannot remove {}: {error}", path.display()))?;
        println!("removed stale {}", path.display());
    }
    Ok(())
}

/// Write only when the content changed, so an unchanged run leaves timestamps alone
/// and does not trigger a rebuild.
fn write_if_changed(path: &Path, contents: &str) -> Result<(), String> {
    if std::fs::read_to_string(path).is_ok_and(|existing| existing == contents) {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }

    std::fs::write(path, contents)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn load(path: &Path) -> Result<Manifest, String> {
    Manifest::load(path).map_err(|error| error.to_string())
}

/// The `src-tauri` directory belonging to a manifest.
fn tauri_directory(manifest: &Path) -> PathBuf {
    manifest
        .parent()
        .unwrap_or(Path::new("."))
        .join("src-tauri")
}

/// Every `app.toml` in the workspace.
///
/// A derivative has one at its root; Origin's own repository has one per example.
pub(crate) fn find_manifests(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut manifests = Vec::new();
    collect_manifests(root, &mut manifests)?;
    manifests.sort();
    Ok(manifests)
}

fn collect_manifests(directory: &Path, found: &mut Vec<PathBuf>) -> Result<(), String> {
    let manifest = directory.join("app.toml");
    if manifest.is_file() {
        found.push(manifest);
    }

    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        // `fixtures` holds projects frozen at old versions, `templates` holds
        // placeholders — neither is a project this workspace maintains.
        if matches!(
            name.as_str(),
            "target" | "node_modules" | "dist" | "gen" | ".git" | "plan" | "fixtures" | "templates"
        ) {
            continue;
        }

        collect_manifests(&path, found)?;
    }

    Ok(())
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}
