//! Machine-checked architecture rules.
//!
//! Documented rules rot. These are the subset of `ARCHITECTURE.md` that can be
//! verified mechanically, so a violation fails CI instead of surviving review.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// Product names that must never appear inside the platform crates (rule 2).
const PRODUCT_NAMES: &[&str] = &["contoso", "fabrikam", "northwind"];

/// Capability grants that hand the frontend far more than it needs (rule 14).
const FORBIDDEN_PERMISSIONS: &[&str] = &[
    "fs:allow-all",
    "fs:default",
    "shell:allow-execute",
    "shell:allow-spawn",
    "shell:default",
];

pub fn run(root: &Path) -> Result<(), String> {
    let root = root.to_path_buf();
    let mut failures = Vec::new();

    failures.extend(check_layer_dependencies(&root)?);
    failures.extend(check_no_product_names(&root)?);
    failures.extend(check_invoke_is_confined(&root)?);
    failures.extend(check_capabilities(&root)?);
    failures.extend(check_commands_exist(&root)?);
    failures.extend(check_manifest_matches_tauri_config(&root)?);

    if failures.is_empty() {
        println!("architecture rules: ok");
        return Ok(());
    }

    let mut report = String::from("architecture violations:\n");
    for failure in &failures {
        let _ = writeln!(report, "  - {failure}");
    }
    Err(report)
}

/// Rule 1, 5: dependencies point downwards only.
fn check_layer_dependencies(root: &Path) -> Result<Vec<String>, String> {
    let mut failures = Vec::new();

    for (layer, forbidden) in [
        ("crates", &["tauri", "adapters", "host", "examples"][..]),
        ("adapters", &["adapters", "examples"][..]),
        ("host", &["examples"][..]),
    ] {
        for manifest in crates_in(&root.join(layer))? {
            let dependencies = dependency_names(&manifest)?;
            let package = manifest
                .parent()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();

            for dependency in &dependencies {
                // `adapters` may of course depend on themselves being in that layer;
                // what is forbidden is depending on a *different* adapter.
                if layer == "adapters" && dependency == &package {
                    continue;
                }

                let violates = match layer {
                    "crates" => {
                        dependency.starts_with("tauri")
                            || is_workspace_member_of(root, dependency, forbidden)
                    }
                    _ => is_workspace_member_of(root, dependency, forbidden),
                };

                if violates {
                    failures.push(format!(
                        "{layer}/{package} depends on `{dependency}` — {layer} must not \
                         depend on {}",
                        forbidden.join(", ")
                    ));
                }
            }
        }
    }

    Ok(failures)
}

/// Rule 2: the platform never knows which product it serves.
///
/// `origin-xtask` is exempt: it is a build tool, not runtime platform code, and it has
/// to name the products in order to check for them.
fn check_no_product_names(root: &Path) -> Result<Vec<String>, String> {
    let mut failures = Vec::new();
    let tooling = root.join("crates").join("origin-xtask");

    for file in rust_sources(&root.join("crates"))? {
        if file.starts_with(&tooling) {
            continue;
        }

        let contents = read(&file)?.to_lowercase();
        for product in PRODUCT_NAMES {
            // The rule itself is quoted in ARCHITECTURE.md and in ADR text; only code
            // is checked here.
            if contents.contains(product) {
                failures.push(format!(
                    "{} mentions the product `{product}` — platform code must not know \
                     its consumers",
                    relative(root, &file)
                ));
            }
        }
    }

    Ok(failures)
}

/// Rule 15: only `@origin/client` speaks Tauri IPC.
///
/// Scans the whole project, not a fixed list of directories: in a derivative the
/// frontend lives wherever that product put it, and a rule that silently skips it is
/// worse than no rule.
fn check_invoke_is_confined(root: &Path) -> Result<Vec<String>, String> {
    let mut failures = Vec::new();
    // The one package allowed to know the transport. In a derivative it arrives
    // through node_modules, which is never scanned.
    let allowed = root.join("frontend").join("client");

    for file in frontend_sources(root)? {
        if file.starts_with(&allowed) {
            continue;
        }

        let contents = read(&file)?;
        if contents.contains("@tauri-apps/api") {
            failures.push(format!(
                "{} imports `@tauri-apps/api` — go through `@origin/client` instead \
                 (ADR-0010)",
                relative(root, &file)
            ));
        }
    }

    Ok(failures)
}

/// Rule 14: least privilege in every capability file, wherever it lives.
fn check_capabilities(root: &Path) -> Result<Vec<String>, String> {
    let mut failures = Vec::new();

    for file in files_with_extension(root, "json")? {
        if !file
            .parent()
            .is_some_and(|parent| parent.ends_with("capabilities"))
        {
            continue;
        }

        let contents = read(&file)?;
        for permission in FORBIDDEN_PERMISSIONS {
            if contents.contains(permission) {
                failures.push(format!(
                    "{} grants `{permission}` — that is not least privilege (ADR-0007)",
                    relative(root, &file)
                ));
            }
        }
    }

    Ok(failures)
}

/// `app.toml` and `tauri.conf.json` must agree on who this product is.
///
/// They are edited in different situations — a version bump here, a window tweak there
/// — and a mismatch is invisible until a release ships under the wrong version or an
/// installer collides with another application's identifier.
fn check_manifest_matches_tauri_config(root: &Path) -> Result<Vec<String>, String> {
    let mut failures = Vec::new();

    for manifest_path in crate::find_manifests(root)? {
        let project = manifest_path.parent().unwrap_or(root);
        let config_path = project.join("src-tauri").join("tauri.conf.json");

        let Ok(config) = read(&config_path) else {
            continue;
        };
        let config: serde_json::Value = match serde_json::from_str(&config) {
            Ok(config) => config,
            Err(error) => {
                failures.push(format!("{}: {error}", relative(root, &config_path)));
                continue;
            }
        };

        let manifest = match origin_manifest::Manifest::load(&manifest_path) {
            Ok(manifest) => manifest,
            Err(error) => {
                failures.push(error.to_string());
                continue;
            }
        };

        for (field, expected, actual) in [
            ("identifier", &manifest.product.id, config.get("identifier")),
            (
                "productName",
                &manifest.product.name,
                config.get("productName"),
            ),
            ("version", &manifest.product.version, config.get("version")),
        ] {
            let actual = actual
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if actual != expected {
                failures.push(format!(
                    "{}: `{field}` is `{actual}`, but app.toml says `{expected}`",
                    relative(root, &config_path)
                ));
            }
        }
    }

    Ok(failures)
}

/// Rule 15, second half: every command the frontend calls must exist in Rust.
///
/// The name is a string on one side and a function on the other, so nothing else
/// catches a typo or a renamed command until it fails at runtime in front of a user.
fn check_commands_exist(root: &Path) -> Result<Vec<String>, String> {
    let mut defined = Vec::new();
    for directory in [
        root.join("host"),
        root.join("examples"),
        root.join("crates"),
    ] {
        for file in rust_sources(&directory)? {
            let contents = read(&file)?;
            defined.extend(tauri_command_names(&contents));
        }
    }

    let mut failures = Vec::new();
    for file in frontend_sources(&root.join("frontend"))?
        .into_iter()
        .chain(frontend_sources(&root.join("examples"))?)
    {
        let contents = read(&file)?;
        for name in invoked_command_names(&contents) {
            if !defined.contains(&name) {
                failures.push(format!(
                    "{} calls the command `{name}`, which no `#[tauri::command]` defines",
                    relative(root, &file)
                ));
            }
        }
    }

    Ok(failures)
}

/// Names of functions annotated with `#[tauri::command]`.
fn tauri_command_names(contents: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut annotated = false;

    for line in contents.lines() {
        let line = line.trim();

        if line.starts_with("#[tauri::command") {
            annotated = true;
            continue;
        }

        if annotated {
            if line.starts_with("#[") {
                continue;
            }
            if let Some(name) = line
                .split("fn ")
                .nth(1)
                .and_then(|rest| rest.split(['(', '<', ' ']).next())
            {
                names.push(name.to_owned());
            }
            annotated = false;
        }
    }

    names
}

/// Command names passed to the client's `command("...")` helper.
fn invoked_command_names(contents: &str) -> Vec<String> {
    contents
        .match_indices("command<")
        .filter_map(|(index, _)| {
            let rest = &contents[index..];
            let open = rest.find('"')?;
            let close = rest[open + 1..].find('"')?;
            Some(rest[open + 1..open + 1 + close].to_owned())
        })
        .collect()
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Whether `name` is a crate living in one of `layers`.
fn is_workspace_member_of(root: &Path, name: &str, layers: &[&str]) -> bool {
    layers.iter().any(|layer| {
        let direct = root.join(layer).join(name).join("Cargo.toml").exists();
        // `examples/demo/src-tauri` does not follow the flat layout.
        let nested =
            root.join(layer).join("demo").join("src-tauri").exists() && name == "origin-demo";
        direct || (*layer == "examples" && nested)
    })
}

fn crates_in(directory: &Path) -> Result<Vec<PathBuf>, String> {
    if !directory.exists() {
        return Ok(Vec::new());
    }

    let mut manifests = Vec::new();
    for entry in read_dir(directory)? {
        let manifest = entry.join("Cargo.toml");
        if manifest.exists() {
            manifests.push(manifest);
        }
    }
    Ok(manifests)
}

fn dependency_names(manifest: &Path) -> Result<Vec<String>, String> {
    let value: toml::Value = toml::from_str(&read(manifest)?)
        .map_err(|error| format!("{}: {error}", manifest.display()))?;

    let mut names = Vec::new();
    for table in ["dependencies", "build-dependencies"] {
        if let Some(toml::Value::Table(entries)) = value.get(table) {
            names.extend(entries.keys().cloned());
        }
    }
    Ok(names)
}

fn rust_sources(directory: &Path) -> Result<Vec<PathBuf>, String> {
    files_with_extension(directory, "rs")
}

fn frontend_sources(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = files_with_extension(directory, "ts")?;
    files.extend(files_with_extension(directory, "svelte")?);
    Ok(files)
}

fn files_with_extension(directory: &Path, extension: &str) -> Result<Vec<PathBuf>, String> {
    if !directory.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in read_dir(directory)? {
        let name = entry
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if entry.is_dir() {
            // Never walk into build output or installed packages.
            if matches!(
                name.as_str(),
                "node_modules" | "target" | "dist" | "gen" | ".git"
            ) {
                continue;
            }
            files.extend(files_with_extension(&entry, extension)?);
        } else if entry.extension().is_some_and(|found| found == extension) {
            files.push(entry);
        }
    }
    Ok(files)
}

fn read_dir(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut entries: Vec<PathBuf> = fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    Ok(entries)
}

fn read(file: &Path) -> Result<String, String> {
    fs::read_to_string(file).map_err(|error| format!("cannot read {}: {error}", file.display()))
}

fn relative(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .into_owned()
}
