//! Where an application keeps its data.
//!
//! Computed here rather than taken from the desktop host, so that a GUI run and a
//! headless run of the same product agree by construction. Two code paths deriving the
//! same directory independently is how a headless mode ends up looking at an empty
//! database.

use origin_domain::{AppError, Result};
use std::path::PathBuf;

/// Overrides the location entirely. Used by tests, and by anyone running a portable
/// installation.
pub const DATA_DIR_ENV: &str = "ORIGIN_DATA_DIR";

/// The directory for `app_id`, created if it does not exist.
///
/// | Platform | Location |
/// | --- | --- |
/// | macOS | `~/Library/Application Support/<app_id>` |
/// | Windows | `%APPDATA%\<app_id>` |
/// | Linux | `$XDG_DATA_HOME/<app_id>`, else `~/.local/share/<app_id>` |
pub fn data_dir(app_id: &str) -> Result<PathBuf> {
    let directory = resolve(
        app_id,
        std::env::var_os(DATA_DIR_ENV).map(PathBuf::from),
        base_dir()?,
    );

    std::fs::create_dir_all(&directory).map_err(|error| {
        AppError::storage(format!("cannot create {}: {error}", directory.display()))
    })?;

    Ok(directory)
}

/// The location decision, without touching the environment or the filesystem.
fn resolve(app_id: &str, override_path: Option<PathBuf>, base: PathBuf) -> PathBuf {
    override_path.unwrap_or_else(|| base.join(app_id))
}

#[cfg(target_os = "macos")]
fn base_dir() -> Result<PathBuf> {
    Ok(home()?.join("Library").join("Application Support"))
}

#[cfg(target_os = "windows")]
fn base_dir() -> Result<PathBuf> {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| AppError::configuration("APPDATA is not set"))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn base_dir() -> Result<PathBuf> {
    match std::env::var_os("XDG_DATA_HOME") {
        Some(path) => Ok(PathBuf::from(path)),
        None => Ok(home()?.join(".local").join("share")),
    }
}

#[cfg(not(target_os = "windows"))]
fn home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| AppError::configuration("HOME is not set"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_override_replaces_the_location_entirely() {
        let resolved = resolve(
            "dev.origin.test",
            Some(PathBuf::from("/tmp/portable")),
            PathBuf::from("/home/user/.local/share"),
        );

        assert_eq!(resolved, PathBuf::from("/tmp/portable"));
    }

    #[test]
    fn the_default_location_carries_the_application_id() {
        let resolved = resolve("dev.origin.test", None, PathBuf::from("/base"));

        assert_eq!(resolved, PathBuf::from("/base/dev.origin.test"));
    }

    #[test]
    fn resolving_creates_the_directory() {
        let target = std::env::temp_dir().join(format!("origin-paths-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&target);

        let resolved = data_dir(target.to_str().expect("utf-8 path"));

        // Without an override the id is appended to the platform base directory, so
        // this only asserts that whatever came back exists.
        assert!(resolved.expect("resolve").is_dir());
    }
}
