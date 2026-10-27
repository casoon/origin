//! Publishing how to reach an endpoint, and reading it back.
//!
//! The port is chosen at runtime, so a client cannot know it in advance. The GUI
//! writes this file when it starts the HTTP adapter; a headless start reads it to
//! discover that a GUI is already serving (G17).

use origin_domain::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// How a client reaches a running instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Discovery {
    /// Full URL, e.g. `http://127.0.0.1:54321/mcp`.
    pub url: String,
    /// The bearer token the endpoint expects (G19).
    pub token: String,
}

impl Discovery {
    /// Write the file atomically (write then rename) so a reader never sees a
    /// half-written file.
    pub fn write(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                AppError::storage(format!("cannot create {}: {error}", parent.display()))
            })?;
        }

        let encoded = serde_json::to_string_pretty(self)
            .map_err(|error| AppError::storage(format!("cannot encode discovery: {error}")))?;

        let temporary = path.with_extension("json.tmp");
        write_private(&temporary, encoded.as_bytes()).map_err(|error| {
            AppError::storage(format!("cannot write {}: {error}", temporary.display()))
        })?;
        std::fs::rename(&temporary, path).map_err(|error| {
            AppError::storage(format!("cannot publish {}: {error}", path.display()))
        })?;

        tracing::debug!(path = %path.display(), "mcp http endpoint published");
        Ok(())
    }

    /// Read the file, or `None` when it does not exist or is unreadable.
    ///
    /// A stale or corrupt file is not an error: the caller simply falls back to
    /// starting its own transport.
    pub fn read(path: &Path) -> Option<Self> {
        let contents = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    /// Remove the file. Missing is fine.
    pub fn remove(path: &Path) {
        let _ = std::fs::remove_file(path);
    }
}

/// Write a file only the current user can read: it carries the bearer token, and the
/// data directory is not guaranteed to be private (on Linux it usually is not).
#[cfg(unix)]
fn write_private(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    // `mode` only applies when the file is created, and a temp file left behind by a
    // crash may carry wider permissions — so always start from a fresh file.
    let _ = std::fs::remove_file(path);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(contents)
}

/// On Windows the file inherits the ACL of the per-user `%APPDATA%` directory, which is
/// already private to the user.
#[cfg(not(unix))]
fn write_private(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let unique = format!("origin-mcp-discovery-{}-{name}.json", std::process::id());
        std::env::temp_dir().join(unique)
    }

    #[test]
    fn a_discovery_round_trips() {
        let path = temp_path("round-trip");
        let discovery = Discovery {
            url: "http://127.0.0.1:5000/mcp".to_owned(),
            token: "abc".to_owned(),
        };

        discovery.write(&path).unwrap();
        let read = Discovery::read(&path).expect("must read back");

        assert_eq!(read, discovery);
        Discovery::remove(&path);
        assert!(Discovery::read(&path).is_none());
    }

    #[test]
    fn a_missing_file_reads_as_none() {
        assert!(Discovery::read(&temp_path("missing")).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn only_the_owner_can_read_the_token() {
        use std::os::unix::fs::PermissionsExt;

        let path = temp_path("permissions");
        Discovery {
            url: "http://127.0.0.1:5000/mcp".to_owned(),
            token: "abc".to_owned(),
        }
        .write(&path)
        .unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        Discovery::remove(&path);
        assert_eq!(mode & 0o777, 0o600);
    }
}
