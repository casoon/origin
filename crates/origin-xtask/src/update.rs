//! `cargo xtask update` — bring a project up to this Origin version (ADR-0025).
//!
//! The point of the whole exercise: a platform improvement reaches four derivatives as
//! four command runs, not four afternoons.

use crate::migrations::{self, Context, Steps};
use crate::version::Version;
use crate::{find_manifests, generate};
use origin_manifest::Manifest;
use std::path::Path;

/// Run every pending migration, regenerate, and record the new version.
pub fn run(root: &Path, dry_run: bool) -> Result<(), String> {
    let manifests = find_manifests(root)?;
    if manifests.is_empty() {
        return Err(format!("no app.toml found under {}", root.display()));
    }

    for manifest_path in manifests {
        let manifest = Manifest::load(&manifest_path).map_err(|error| error.to_string())?;
        let project = manifest_path.parent().unwrap_or(root);
        let from = Version::parse(&manifest.origin.version)?;

        println!("\n{}", manifest_path.display());
        println!("  origin {from} → {}", migrations::CURRENT);

        if from > migrations::CURRENT {
            return Err(format!(
                "{} tracks origin {from}, but this build only knows up to {}. \
                 Upgrade the origin dependency instead of downgrading the project.",
                manifest_path.display(),
                migrations::CURRENT
            ));
        }

        if from == migrations::CURRENT {
            println!("  already up to date");
            continue;
        }

        let context = Context {
            project,
            manifest: &manifest,
            dry_run,
        };
        let steps = migrations::apply(&context, from)?;

        if !dry_run {
            // Regeneration comes last: a migration may have changed what the manifest
            // describes, and generated files must reflect the final state.
            generate::run(root)?;
            set_version(&manifest_path, migrations::CURRENT)?;
        }

        report(&steps, dry_run);
    }

    Ok(())
}

fn report(steps: &Steps, dry_run: bool) {
    if dry_run {
        println!("\n  dry run — nothing was written");
    }

    for changed in &steps.changed {
        println!("  ✓ {changed}");
    }

    for skipped in &steps.skipped {
        println!("  – skipped: {skipped}");
    }

    if !steps.manual.is_empty() {
        println!("\n  Manual steps required:");
        for manual in &steps.manual {
            println!("  → {manual}");
        }
    }
}

/// Write the new version into `app.toml`.
///
/// Format-preserving: the manifest is a file a human wrote and will read again, so
/// comments and layout survive.
fn set_version(manifest_path: &Path, version: Version) -> Result<(), String> {
    let contents = std::fs::read_to_string(manifest_path)
        .map_err(|error| format!("cannot read {}: {error}", manifest_path.display()))?;

    let mut document: toml_edit::DocumentMut = contents
        .parse()
        .map_err(|error| format!("{} is not valid TOML: {error}", manifest_path.display()))?;

    document["origin"]["version"] = toml_edit::value(version.to_string());

    std::fs::write(manifest_path, document.to_string())
        .map_err(|error| format!("cannot write {}: {error}", manifest_path.display()))
}
