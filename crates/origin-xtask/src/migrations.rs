//! Project migrations (ADR-0025).
//!
//! Library code updates through Cargo. Project structure and configuration cannot —
//! that is what these are for. They work like database migrations: forward-only,
//! ordered, idempotent, and each one produces a normal reviewable diff rather than a
//! black-box result.
//!
//! What a migration must never do is touch product-owned files (ADR-0022). Such steps
//! are reported as a checklist instead.

use crate::version::Version;
use origin_manifest::{Manifest, SecurityProfile};
use std::path::{Path, PathBuf};

/// The Origin version this build migrates projects to.
pub const CURRENT: Version = Version::new(0, 2, 0);

/// What a migration did, and what it could not do by itself.
#[derive(Debug, Default)]
pub struct Steps {
    /// Files the migration changed.
    pub changed: Vec<String>,
    /// Work only a human can do, printed as a checklist.
    pub manual: Vec<String>,
    /// Left alone because the project declared an override (§46).
    pub skipped: Vec<String>,
}

impl Steps {
    fn merge(&mut self, other: Steps) {
        self.changed.extend(other.changed);
        self.manual.extend(other.manual);
        self.skipped.extend(other.skipped);
    }
}

pub struct Context<'a> {
    /// Directory holding `app.toml`.
    pub project: &'a Path,
    pub manifest: &'a Manifest,
    /// Set for `--dry-run`: report what would change, write nothing.
    pub dry_run: bool,
}

impl Context<'_> {
    fn tauri_directory(&self) -> PathBuf {
        self.project.join("src-tauri")
    }
}

pub struct Migration {
    /// Version this migration brings a project *to*.
    pub to: Version,
    pub summary: &'static str,
    pub apply: fn(&Context) -> Result<Steps, String>,
}

/// Every migration, oldest first.
pub fn all() -> Vec<Migration> {
    vec![Migration {
        to: Version::new(0, 2, 0),
        summary: "capability files are generated from app.toml",
        apply: adopt_generated_capabilities,
    }]
}

/// Migrations a project at `from` still has to run.
pub fn pending(from: Version) -> Vec<Migration> {
    all().into_iter().filter(|m| m.to > from).collect()
}

/// Apply every pending migration in order.
pub fn apply(context: &Context, from: Version) -> Result<Steps, String> {
    let mut steps = Steps::default();

    for migration in pending(from) {
        println!("  {} → {}", from, migration.to);
        println!("    {}", migration.summary);
        steps.merge((migration.apply)(context)?);
    }

    Ok(steps)
}

// ---------------------------------------------------------------------------
// 0.1.0 → 0.2.0
// ---------------------------------------------------------------------------

/// Convert hand-written Tauri capability files into a security profile in `app.toml`.
///
/// This is the migration the first real derivative needs: an application that predates
/// Origin has one `capabilities/default.json` it has been widening a line at a time.
fn adopt_generated_capabilities(context: &Context) -> Result<Steps, String> {
    let mut steps = Steps::default();
    let capabilities = context.tauri_directory().join("capabilities");

    if !capabilities.is_dir() {
        return Ok(steps);
    }

    if context.manifest.has_override("hand_written_capabilities") {
        steps.skipped.push(format!(
            "{} — the project declares `hand_written_capabilities`",
            display(context.project, &capabilities)
        ));
        return Ok(steps);
    }

    for path in hand_written_files(&capabilities)? {
        let contents = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;

        let capability: serde_json::Value = serde_json::from_str(&contents)
            .map_err(|error| format!("{} is not valid JSON: {error}", path.display()))?;

        let granted = string_list(&capability, "permissions");
        let windows = string_list(&capability, "windows");

        match narrowest_profile(&granted) {
            Some(profile) => {
                // The manifest is the source now; the file is generated from it.
                if !context.dry_run {
                    std::fs::remove_file(&path)
                        .map_err(|error| format!("cannot remove {}: {error}", path.display()))?;
                }

                steps.changed.push(format!(
                    "{} → replaced by profile `{}` for window(s) {}",
                    display(context.project, &path),
                    profile.identifier(),
                    if windows.is_empty() {
                        "main".to_owned()
                    } else {
                        windows.join(", ")
                    }
                ));

                for window in windows
                    .iter()
                    .filter(|window| !context.manifest.security.windows.contains_key(*window))
                {
                    steps.manual.push(format!(
                        "add to app.toml:  [security.windows]  {window} = {{ profile = \"{}\" }}",
                        profile.identifier()
                    ));
                }
            }

            // Widening a profile to fit is exactly the decision a migration must not
            // make on someone's behalf.
            None => steps.manual.push(format!(
                "{} grants permissions no profile covers ({}). Choose a profile in \
                 app.toml, or declare `hand_written_capabilities = true` under \
                 [origin.overrides].",
                display(context.project, &path),
                beyond_every_profile(&granted).join(", ")
            )),
        }
    }

    Ok(steps)
}

/// The narrowest profile that covers every granted permission.
fn narrowest_profile(granted: &[String]) -> Option<SecurityProfile> {
    let mut candidates = [
        SecurityProfile::ReadonlyDashboard,
        SecurityProfile::StandardDashboard,
        SecurityProfile::AccountSettings,
    ];
    candidates.sort_by_key(|profile| profile.permissions().len());

    candidates.into_iter().find(|profile| {
        granted
            .iter()
            .all(|permission| profile.permissions().contains(&permission.as_str()))
    })
}

/// Permissions no profile grants — what a human has to decide about.
fn beyond_every_profile(granted: &[String]) -> Vec<String> {
    let known: Vec<&str> = [
        SecurityProfile::ReadonlyDashboard,
        SecurityProfile::StandardDashboard,
        SecurityProfile::AccountSettings,
    ]
    .iter()
    .flat_map(|profile| profile.permissions().iter().copied())
    .collect();

    granted
        .iter()
        .filter(|permission| !known.contains(&permission.as_str()))
        .cloned()
        .collect()
}

/// Capability files that were not produced by the generator.
fn hand_written_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;

    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .filter(|path| {
            !std::fs::read_to_string(path)
                .unwrap_or_default()
                .contains("Generated from app.toml")
        })
        .collect();

    files.sort();
    Ok(files)
}

fn string_list(value: &serde_json::Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_narrow_capability_maps_to_the_narrowest_profile() {
        let granted = vec!["core:default".to_owned()];

        assert_eq!(
            narrowest_profile(&granted),
            Some(SecurityProfile::ReadonlyDashboard)
        );
    }

    #[test]
    fn a_capability_needing_more_maps_to_a_wider_profile() {
        let granted = vec![
            "core:default".to_owned(),
            "core:window:allow-close".to_owned(),
        ];

        assert_eq!(
            narrowest_profile(&granted),
            Some(SecurityProfile::AccountSettings)
        );
    }

    #[test]
    fn filesystem_access_maps_to_no_profile_at_all() {
        let granted = vec![
            "core:default".to_owned(),
            "fs:allow-write-text-file".to_owned(),
        ];

        assert_eq!(narrowest_profile(&granted), None);
        assert_eq!(
            beyond_every_profile(&granted),
            vec!["fs:allow-write-text-file"]
        );
    }

    #[test]
    fn only_migrations_newer_than_the_project_are_pending() {
        assert_eq!(pending(Version::new(0, 1, 0)).len(), 1);
        assert!(pending(CURRENT).is_empty());
        assert!(pending(Version::new(9, 0, 0)).is_empty());
    }
}
