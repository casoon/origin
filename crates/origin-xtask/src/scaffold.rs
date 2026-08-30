//! `cargo xtask new` — a new application from the template.
//!
//! What the template gives a product on day one is not a blank window: it is the
//! architecture contract, logging, error handling, settings, secrets, storage, the
//! security profile, CI, and a module that is already testable without a desktop
//! session.

use crate::migrations::CURRENT;
use std::path::{Path, PathBuf};

/// Files copied verbatim; everything else is treated as text and substituted.
const BINARY_EXTENSIONS: &[&str] = &["png", "ico", "icns", "jpg", "jpeg", "webp"];

#[derive(Debug)]
pub struct Options {
    /// Directory and crate name, e.g. `my-app`.
    pub slug: String,
    /// Human-readable product name.
    pub name: String,
    /// Reverse-DNS identifier.
    pub id: String,
    /// Where to create the project.
    pub into: PathBuf,
    /// Point the dependencies at this Origin checkout instead of at crates.io.
    ///
    /// Used by Origin's own CI to prove the template still builds against `main`, and
    /// useful while developing the platform and a product side by side.
    pub local: Option<PathBuf>,
}

pub fn run(root: &Path, options: &Options) -> Result<(), String> {
    validate_options(options)?;

    let template = root.join("templates").join("app");
    if !template.is_dir() {
        return Err(format!(
            "no template at {} — scaffolding currently needs the Origin repository",
            template.display()
        ));
    }

    let target = options.into.join(&options.slug);
    if target.exists() {
        return Err(format!("{} already exists", target.display()));
    }

    let substitutions = substitutions(options);
    copy_template(&template, &target, &substitutions)?;

    if let Some(origin) = &options.local {
        patch_to_local(&target, origin)?;
    }

    // Leave a project that already passes its own checks, rather than one whose first
    // CI run is red.
    crate::generate(&target)?;

    println!("created {}", target.display());
    println!("\nnext:");
    println!("  cd {}", target.display());
    println!("  pnpm install");
    println!("  cargo tauri dev");
    println!("\nReplace the placeholder icons in src-tauri/icons before shipping.");
    Ok(())
}

fn validate_options(options: &Options) -> Result<(), String> {
    let valid_slug = !options.slug.is_empty()
        && !options.slug.starts_with('-')
        && !options.slug.ends_with('-')
        && !options.slug.contains("--")
        && options
            .slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if !valid_slug {
        return Err(format!(
            "invalid slug `{}` — use lowercase ASCII letters, digits and single path-safe hyphens",
            options.slug
        ));
    }

    if options.name.trim().is_empty()
        || options
            .name
            .chars()
            .any(|character| character.is_control() || matches!(character, '"' | '\\'))
    {
        return Err(
            "product name must be non-empty and contain no control characters, quotes or backslashes"
                .to_owned(),
        );
    }

    let labels: Vec<&str> = options.id.split('.').collect();
    let valid_id = labels.len() >= 2
        && labels.iter().all(|label| {
            !label.is_empty()
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && label
                    .bytes()
                    .next()
                    .is_some_and(|byte| byte.is_ascii_alphanumeric())
                && label
                    .bytes()
                    .next_back()
                    .is_some_and(|byte| byte.is_ascii_alphanumeric())
        });
    if !valid_id {
        return Err(format!(
            "invalid product id `{}` — expected a reverse-DNS identifier",
            options.id
        ));
    }

    Ok(())
}

fn substitutions(options: &Options) -> Vec<(&'static str, String)> {
    vec![
        ("__PRODUCT_ID__", options.id.clone()),
        ("__PRODUCT_NAME__", options.name.clone()),
        ("__CRATE_NAME_SNAKE__", options.slug.replace('-', "_")),
        ("__CRATE_NAME__", options.slug.clone()),
        ("__PACKAGE_NAME__", options.slug.clone()),
        ("__ORIGIN_VERSION__", CURRENT.to_string()),
        (
            "__ORIGIN_SEMVER__",
            format!("{}.{}", CURRENT.major, CURRENT.minor),
        ),
        ("__ORIGIN_NPM__", format!("^{CURRENT}")),
    ]
}

fn copy_template(
    template: &Path,
    target: &Path,
    substitutions: &[(&str, String)],
) -> Result<(), String> {
    std::fs::create_dir_all(target)
        .map_err(|error| format!("cannot create {}: {error}", target.display()))?;

    let entries = std::fs::read_dir(template)
        .map_err(|error| format!("cannot read {}: {error}", template.display()))?;

    for entry in entries.filter_map(Result::ok) {
        let source = entry.path();
        let destination = target.join(entry.file_name());

        if source.is_dir() {
            copy_template(&source, &destination, substitutions)?;
            continue;
        }

        let is_binary = source
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| BINARY_EXTENSIONS.contains(&extension));

        if is_binary {
            std::fs::copy(&source, &destination)
                .map_err(|error| format!("cannot copy {}: {error}", source.display()))?;
            continue;
        }

        let contents = std::fs::read_to_string(&source)
            .map_err(|error| format!("cannot read {}: {error}", source.display()))?;

        // Longest placeholder first, so `__CRATE_NAME_SNAKE__` is not eaten by
        // `__CRATE_NAME__`.
        let mut rendered = contents;
        for (placeholder, value) in substitutions {
            rendered = rendered.replace(placeholder, value);
        }

        std::fs::write(&destination, rendered)
            .map_err(|error| format!("cannot write {}: {error}", destination.display()))?;
    }

    Ok(())
}

/// Point the generated project at a local Origin checkout.
///
/// Rewrites the dependencies rather than adding `[patch.crates-io]`: patching only
/// works for crates that exist in the registry, and the Origin crates are not published
/// yet. Used by Origin's own CI to prove the template still builds against `main`.
fn patch_to_local(target: &Path, origin: &Path) -> Result<(), String> {
    let origin = origin
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", origin.display()))?;

    rewrite_cargo_dependencies(target, &origin)?;
    rewrite_npm_dependencies(target, &origin)
}

/// Where each Origin crate lives inside the repository.
fn crate_location(name: &str) -> &'static str {
    match name {
        "origin-tauri" => "host",
        "origin-http-reqwest"
        | "origin-notifications-tauri"
        | "origin-secrets-system"
        | "origin-storage-sqlite"
        | "origin-auth-loopback" => "adapters",
        _ => "crates",
    }
}

fn rewrite_cargo_dependencies(target: &Path, origin: &Path) -> Result<(), String> {
    let manifest = target.join("Cargo.toml");
    let contents = std::fs::read_to_string(&manifest)
        .map_err(|error| format!("cannot read {}: {error}", manifest.display()))?;

    let mut document: toml_edit::DocumentMut = contents
        .parse()
        .map_err(|error| format!("{} is not valid TOML: {error}", manifest.display()))?;

    let Some(dependencies) = document
        .get_mut("workspace")
        .and_then(|workspace| workspace.get_mut("dependencies"))
        .and_then(toml_edit::Item::as_table_mut)
    else {
        return Err("template Cargo.toml has no [workspace.dependencies]".to_owned());
    };

    let names: Vec<String> = dependencies
        .iter()
        .map(|(name, _)| name.to_owned())
        .filter(|name| name.starts_with("origin-"))
        .collect();

    for name in names {
        let path = origin.join(crate_location(&name)).join(&name);
        let mut value = toml_edit::InlineTable::new();
        value.insert("path", path.display().to_string().into());
        dependencies[&name] = toml_edit::value(value);
    }

    std::fs::write(&manifest, document.to_string())
        .map_err(|error| format!("cannot write {}: {error}", manifest.display()))
}

fn rewrite_npm_dependencies(target: &Path, origin: &Path) -> Result<(), String> {
    let manifest = target.join("ui").join("package.json");
    let contents = std::fs::read_to_string(&manifest)
        .map_err(|error| format!("cannot read {}: {error}", manifest.display()))?;

    let client_release = format!("\"@origin/client\": \"^{CURRENT}\"");
    let ui_release = format!("\"@origin/ui\": \"^{CURRENT}\"");
    let rewritten = contents
        .replace(
            &client_release,
            &format!(
                "\"@origin/client\": \"link:{}\"",
                origin.join("frontend").join("client").display()
            ),
        )
        .replace(
            &ui_release,
            &format!(
                "\"@origin/ui\": \"link:{}\"",
                origin.join("frontend").join("ui").display()
            ),
        );

    std::fs::write(&manifest, rewritten)
        .map_err(|error| format!("cannot write {}: {error}", manifest.display()))
}
