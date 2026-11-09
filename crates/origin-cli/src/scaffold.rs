//! `origin new` — a new application from the embedded template.
//!
//! What the template gives a product on day one is not a blank window: it is the
//! architecture contract, logging, error handling, settings, secrets, storage, the
//! security profile, CI, and a module that is already testable without a desktop
//! session.

use std::path::{Path, PathBuf};

use origin_xtask::CURRENT_VERSION;

/// Files copied verbatim; everything else is treated as text and substituted.
const BINARY_EXTENSIONS: &[&str] = &["png", "ico", "icns", "jpg", "jpeg", "webp"];

/// Suffix carried by a template file whose real name would make `cargo package` skip
/// the directory it lives in — a nested `Cargo.toml` does exactly that, and the
/// template has three. Stripped when the file is written (ADR-0032).
const RENAMED_SUFFIX: &str = ".in";

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

pub fn run(options: &Options) -> Result<(), String> {
    validate_options(options)?;

    let target = options.into.join(&options.slug);
    if target.exists() {
        return Err(format!("{} already exists", target.display()));
    }

    let substitutions = substitutions(options);
    write_template(&target, &substitutions)?;

    if let Some(origin) = &options.local {
        patch_to_local(&target, origin)?;
    }

    // Leave a project that already passes its own checks, rather than one whose first
    // CI run is red.
    origin_xtask::generate(&target)?;

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
        ("__ORIGIN_VERSION__", CURRENT_VERSION.to_string()),
        (
            "__ORIGIN_SEMVER__",
            format!("{}.{}", CURRENT_VERSION.major, CURRENT_VERSION.minor),
        ),
        ("__ORIGIN_NPM__", format!("^{CURRENT_VERSION}")),
    ]
}

/// Write every embedded template file into `target`.
fn write_template(target: &Path, substitutions: &[(&str, String)]) -> Result<(), String> {
    for (path, contents) in crate::template_files() {
        let destination = target.join(destination(path));

        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }

        let rendered = if is_binary(path) {
            (*contents).to_vec()
        } else {
            let text = std::str::from_utf8(contents)
                .map_err(|error| format!("template file {path} is not valid UTF-8: {error}"))?;
            render(text, substitutions).into_bytes()
        };

        std::fs::write(&destination, rendered)
            .map_err(|error| format!("cannot write {}: {error}", destination.display()))?;
    }

    Ok(())
}

/// The path a template file is written to: its own, without the `.in` suffix.
fn destination(path: &str) -> &str {
    path.strip_suffix(RENAMED_SUFFIX).unwrap_or(path)
}

fn is_binary(path: &str) -> bool {
    path.rsplit('.')
        .next()
        .is_some_and(|extension| BINARY_EXTENSIONS.contains(&extension))
}

/// Replace every placeholder in a template file.
///
/// The substitutions are applied in order, longest placeholder first, so
/// `__CRATE_NAME_SNAKE__` is not eaten by `__CRATE_NAME__`.
fn render(contents: &str, substitutions: &[(&str, String)]) -> String {
    let mut rendered = contents.to_owned();
    for (placeholder, value) in substitutions {
        rendered = rendered.replace(placeholder, value);
    }
    rendered
}

/// Point the generated project at a local Origin checkout.
///
/// Rewrites the dependencies rather than adding `[patch.crates-io]`: patching only
/// works only for crates that exist in the registry. Used by Origin's own CI to prove
/// the template still builds against `main`.
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

    let client_release = format!("\"@casoon/origin-client\": \"^{CURRENT_VERSION}\"");
    let ui_release = format!("\"@casoon/origin-ui\": \"^{CURRENT_VERSION}\"");
    let rewritten = contents
        .replace(
            &client_release,
            &format!(
                "\"@casoon/origin-client\": \"link:{}\"",
                origin.join("frontend").join("client").display()
            ),
        )
        .replace(
            &ui_release,
            &format!(
                "\"@casoon/origin-ui\": \"link:{}\"",
                origin.join("frontend").join("ui").display()
            ),
        );

    std::fs::write(&manifest, rewritten)
        .map_err(|error| format!("cannot write {}: {error}", manifest.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_renamed_template_file_is_written_under_its_real_name() {
        assert_eq!(destination("Cargo.toml.in"), "Cargo.toml");
        assert_eq!(destination("xtask/Cargo.toml.in"), "xtask/Cargo.toml");
        assert_eq!(destination("app.toml"), "app.toml");
    }

    #[test]
    fn the_longer_placeholder_wins() {
        let options = Options {
            slug: "my-app".to_owned(),
            name: "My App".to_owned(),
            id: "dev.example.myapp".to_owned(),
            into: PathBuf::from("."),
            local: None,
        };

        let rendered = render(
            "name = \"__CRATE_NAME__\"\nlib = \"__CRATE_NAME_SNAKE___lib\"",
            &substitutions(&options),
        );

        assert_eq!(rendered, "name = \"my-app\"\nlib = \"my_app_lib\"");
    }

    #[test]
    fn icons_are_not_treated_as_text() {
        assert!(is_binary("src-tauri/icons/icon.icns"));
        assert!(is_binary("src-tauri/icons/128x128@2x.png"));
        assert!(!is_binary("src-tauri/tauri.conf.json"));
        assert!(!is_binary("Cargo.toml.in"));
    }

    fn options(slug: &str, id: &str) -> Options {
        Options {
            slug: slug.to_owned(),
            name: "My App".to_owned(),
            id: id.to_owned(),
            into: PathBuf::from("."),
            local: None,
        }
    }

    #[test]
    fn a_slug_that_would_not_survive_a_path_or_a_crate_name_is_refused() {
        assert!(validate_options(&options("my-app", "dev.example.app")).is_ok());
        assert!(validate_options(&options("My-App", "dev.example.app")).is_err());
        assert!(validate_options(&options("../escape", "dev.example.app")).is_err());
        assert!(validate_options(&options("my--app", "dev.example.app")).is_err());
        assert!(validate_options(&options("-app", "dev.example.app")).is_err());
    }

    #[test]
    fn a_product_id_that_is_not_reverse_dns_is_refused() {
        assert!(validate_options(&options("my-app", "app")).is_err());
        assert!(validate_options(&options("my-app", "dev..app")).is_err());
        assert!(validate_options(&options("my-app", "dev.example.")).is_err());
    }
}
