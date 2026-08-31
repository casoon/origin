//! Machine-checked architecture rules.
//!
//! Documented rules rot. These are the subset of `ARCHITECTURE.md` that can be
//! verified mechanically, so a violation fails CI instead of surviving review.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    let dependencies_by_package = workspace_dependencies(root)?;

    for (layer, forbidden) in [
        ("crates", &["tauri", "adapters", "host", "examples"][..]),
        ("adapters", &["adapters", "examples"][..]),
        ("host", &["examples"][..]),
    ] {
        for manifest in crates_in(&root.join(layer))? {
            let package = manifest
                .parent()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let dependencies = dependencies_by_package
                .get(&package)
                .cloned()
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

        let config = match read(&config_path) {
            Ok(config) => config,
            Err(error) => {
                failures.push(error);
                continue;
            }
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
    for file in rust_sources(root)? {
        let contents = read(&file)?;
        defined.extend(tauri_command_names(&contents));
    }

    let mut failures = Vec::new();
    for file in frontend_sources(root)? {
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
    let mut names = Vec::new();
    let mut remaining = contents;

    while let Some(index) = remaining.find("command") {
        remaining = &remaining[index + "command".len()..];
        let Some(arguments) = remaining.find('(') else {
            break;
        };
        let before_arguments = &remaining[..arguments];
        let plain_call = before_arguments.trim().is_empty();
        let generic_call = before_arguments.trim_start().starts_with('<')
            && before_arguments.trim_end().ends_with('>');
        if !(plain_call || generic_call) {
            continue;
        }

        let rest = &remaining[arguments + 1..];
        let rest = rest.trim_start();
        let Some(quote) = rest
            .chars()
            .next()
            .filter(|quote| matches!(quote, '"' | '\''))
        else {
            continue;
        };
        let value = &rest[quote.len_utf8()..];
        let Some(close) = value.find(quote) else {
            continue;
        };
        names.push(value[..close].to_owned());
    }

    names
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

/// Every workspace member's dependency names, keyed by package name.
///
/// Backed by `cargo metadata` rather than a manifest's top-level TOML keys: reading
/// only `[dependencies]` and `[build-dependencies]` missed `[dev-dependencies]`,
/// target-specific tables (`[target.'cfg(...)'.dependencies]`), and reported a renamed
/// dependency's local alias (`desktop = { package = "tauri" }`) instead of the crate it
/// actually names — three ways a layering violation could pass this check unnoticed.
/// `cargo metadata` resolves all of that once, for every member, in one call.
fn workspace_dependencies(root: &Path) -> Result<HashMap<String, Vec<String>>, String> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot run cargo metadata: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cannot parse cargo metadata output: {error}"))?;

    dependencies_from_metadata(&metadata)
}

/// The parsing half of [`workspace_dependencies`], separated so it can be tested
/// against a fixed `cargo metadata` document instead of a real cargo invocation.
fn dependencies_from_metadata(
    metadata: &serde_json::Value,
) -> Result<HashMap<String, Vec<String>>, String> {
    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "cargo metadata: no `packages` array in its output".to_owned())?;

    let mut by_package = HashMap::new();
    for package in packages {
        let Some(name) = package.get("name").and_then(serde_json::Value::as_str) else {
            continue;
        };

        // The dependency's own name, not `rename` — a `package = "..."` alias is a
        // local name for the crate in *this* manifest's code, not a different crate.
        // No filtering on `kind` (normal/dev/build) or `target`: all of them are real
        // dependency edges this check must see.
        let dependencies = package
            .get("dependencies")
            .and_then(serde_json::Value::as_array)
            .map(|dependencies| {
                dependencies
                    .iter()
                    .filter_map(|dependency| {
                        dependency
                            .get("name")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_owned)
                    })
                    .collect()
            })
            .unwrap_or_default();

        by_package.insert(name.to_owned(), dependencies);
    }

    Ok(by_package)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_calls_with_and_without_a_generic_are_found() {
        let source = r#"
            command<Result>("with_result");
            command('without_result');
        "#;

        assert_eq!(
            invoked_command_names(source),
            vec!["with_result", "without_result"]
        );
    }

    /// The three gaps a top-level-TOML-keys check had: a dev-dependency, a
    /// target-specific dependency, and a renamed dependency reported under its real
    /// name rather than its local alias.
    #[test]
    fn dev_target_and_renamed_dependencies_are_all_reported_by_their_real_name() {
        let metadata = serde_json::json!({
            "packages": [
                {
                    "name": "origin-example",
                    "dependencies": [
                        { "name": "origin-core", "rename": null, "kind": null, "target": null },
                        { "name": "tauri", "rename": "desktop", "kind": null, "target": null },
                        { "name": "origin-storage", "rename": null, "kind": "dev", "target": null },
                        {
                            "name": "winapi",
                            "rename": null,
                            "kind": null,
                            "target": "cfg(windows)"
                        }
                    ]
                }
            ]
        });

        let dependencies = dependencies_from_metadata(&metadata).unwrap();

        let mut names = dependencies["origin-example"].clone();
        names.sort();
        assert_eq!(
            names,
            vec!["origin-core", "origin-storage", "tauri", "winapi"]
        );
    }

    #[test]
    fn a_package_with_no_dependencies_field_is_reported_as_having_none() {
        let metadata = serde_json::json!({
            "packages": [{ "name": "origin-leaf" }]
        });

        let dependencies = dependencies_from_metadata(&metadata).unwrap();
        assert_eq!(dependencies["origin-leaf"], Vec::<String>::new());
    }
}
