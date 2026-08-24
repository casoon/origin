//! The app manifest (ADR-0021).
//!
//! `app.toml` answers *what is this product?* The composition root answers *how is it
//! assembled?* Everything that can be derived from the first is generated rather than
//! written by hand — generated files can be updated without merge conflicts, which is
//! what makes an Origin upgrade cheap.
//!
//! The format is deliberately small. Everything in it becomes migration-liable the
//! moment a second product exists.

mod distribution;
mod profile;
mod security;

pub use distribution::{Channel, DistributionSection, Target, UpdaterSection};
pub use profile::SecurityProfile;
pub use security::Capability;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("{path} is not valid TOML: {source}")]
    Parse {
        path: String,
        #[source]
        source: toml::de::Error,
    },

    #[error("{path}: {message}")]
    Invalid { path: String, message: String },
}

/// A parsed `app.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub origin: OriginSection,
    pub product: ProductSection,
    #[serde(default)]
    pub platform: PlatformSection,
    /// Modules the product compiles in. The value switches a module on or off.
    #[serde(default)]
    pub modules: BTreeMap<String, bool>,
    #[serde(default)]
    pub security: SecuritySection,
    #[serde(default)]
    pub distribution: DistributionSection,
}

/// Which Origin version this product tracks.
///
/// Read by `origin update` to decide which migrations still have to run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginSection {
    pub version: String,
    /// Deliberate deviations from an Origin recommendation (§46).
    ///
    /// A migration skips whatever is listed here and reports it as a manual step
    /// instead of overwriting a decision someone made on purpose.
    #[serde(default)]
    pub overrides: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSection {
    /// Reverse-DNS identifier. Also scopes credentials in the OS keychain.
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlatformSection {
    pub tray: bool,
    pub autostart: bool,
    pub notifications: bool,
    pub updater: bool,
    pub single_instance: bool,
    pub window_state: bool,
}

impl Default for PlatformSection {
    fn default() -> Self {
        Self {
            tray: false,
            autostart: false,
            notifications: true,
            updater: false,
            // On by default: two instances of the same desktop app fighting over one
            // database is a bug in every product, not a per-product choice.
            single_instance: true,
            window_state: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecuritySection {
    /// Security profile per window label (ADR-0007).
    #[serde(default)]
    pub windows: BTreeMap<String, WindowSecurity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSecurity {
    pub profile: SecurityProfile,
}

impl Manifest {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let path = path.as_ref();
        let display = path.display().to_string();

        let contents = std::fs::read_to_string(path).map_err(|source| ManifestError::Read {
            path: display.clone(),
            source,
        })?;

        let manifest: Self = toml::from_str(&contents).map_err(|source| ManifestError::Parse {
            path: display.clone(),
            source,
        })?;

        manifest.validate(&display)?;
        Ok(manifest)
    }

    /// Checks that cannot be expressed in the type system.
    fn validate(&self, path: &str) -> Result<(), ManifestError> {
        let invalid = |message: String| ManifestError::Invalid {
            path: path.to_owned(),
            message,
        };

        if self.product.id.split('.').count() < 2 {
            return Err(invalid(format!(
                "product.id must be reverse-DNS, got `{}`",
                self.product.id
            )));
        }

        if self.security.windows.is_empty() {
            return Err(invalid(
                "security.windows is empty — every window needs an explicit security \
                 profile, and a product with no windows cannot be shown"
                    .to_owned(),
            ));
        }

        // An updater without a verifiable endpoint is worse than none: it turns a
        // compromised host into arbitrary code execution on every installation.
        if self.distribution.updater.enabled {
            if self.distribution.updater.endpoints.is_empty() {
                return Err(invalid(
                    "distribution.updater is enabled but declares no endpoints".to_owned(),
                ));
            }

            if let Some(insecure) = self
                .distribution
                .updater
                .endpoints
                .iter()
                .find(|endpoint| !endpoint.starts_with("https://"))
            {
                return Err(invalid(format!(
                    "update endpoint `{insecure}` is not https — an update channel that \
                     can be intercepted is a code execution channel"
                )));
            }
        }

        // A tray application that quits with its last window has no tray to speak of;
        // catching that here is cheaper than a bug report about it.
        if self.platform.tray && !self.platform.single_instance {
            return Err(invalid(
                "platform.tray with single_instance = false: a second instance would add \
                 a second tray icon"
                    .to_owned(),
            ));
        }

        Ok(())
    }

    /// Modules switched on, in a stable order.
    pub fn enabled_modules(&self) -> Vec<&str> {
        self.modules
            .iter()
            .filter(|(_, enabled)| **enabled)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    pub fn has_override(&self, key: &str) -> bool {
        self.overrides_contains(key)
    }

    fn overrides_contains(&self, key: &str) -> bool {
        self.origin.overrides.get(key).copied().unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(extra: &str) -> Result<Manifest, ManifestError> {
        let contents = format!(
            r#"
[origin]
version = "0.1.0"

[product]
id = "dev.origin.demo"
name = "Origin Demo"
version = "0.1.0"

[security.windows.main]
profile = "standard-dashboard"
{extra}
"#
        );

        let parsed: Manifest =
            toml::from_str(&contents).map_err(|source| ManifestError::Parse {
                path: "test".to_owned(),
                source,
            })?;
        parsed.validate("test")?;
        Ok(parsed)
    }

    #[test]
    fn a_minimal_manifest_parses_with_sensible_defaults() {
        let manifest = manifest("").unwrap();

        assert_eq!(manifest.product.id, "dev.origin.demo");
        assert!(manifest.platform.single_instance);
        assert!(manifest.platform.notifications);
        assert!(!manifest.platform.tray);
    }

    #[test]
    fn only_enabled_modules_are_listed_and_the_order_is_stable() {
        let manifest =
            manifest("\n[modules]\npulse = true\nlegacy = false\ninbox = true\n").unwrap();

        assert_eq!(manifest.enabled_modules(), vec!["inbox", "pulse"]);
    }

    #[test]
    fn a_product_id_that_is_not_reverse_dns_is_rejected() {
        let contents = r#"
[origin]
version = "0.1.0"

[product]
id = "demo"
name = "Demo"
version = "0.1.0"

[security.windows.main]
profile = "standard-dashboard"
"#;
        let parsed: Manifest = toml::from_str(contents).unwrap();

        let error = parsed.validate("test").unwrap_err();
        assert!(error.to_string().contains("reverse-DNS"), "got: {error}");
    }

    #[test]
    fn a_window_without_a_security_profile_cannot_exist() {
        let contents = r#"
[origin]
version = "0.1.0"

[product]
id = "dev.origin.demo"
name = "Demo"
version = "0.1.0"
"#;
        let parsed: Manifest = toml::from_str(contents).unwrap();

        let error = parsed.validate("test").unwrap_err();
        assert!(
            error.to_string().contains("security profile"),
            "got: {error}"
        );
    }

    #[test]
    fn a_tray_app_that_allows_second_instances_is_rejected() {
        let error = manifest("\n[platform]\ntray = true\nsingle_instance = false\n").unwrap_err();

        assert!(
            error.to_string().contains("second tray icon"),
            "got: {error}"
        );
    }

    #[test]
    fn overrides_default_to_absent() {
        let manifest = manifest("").unwrap();
        assert!(!manifest.has_override("custom_window_management"));

        let manifest =
            manifest_with_override("\n[origin.overrides]\ncustom_window_management = true\n");
        assert!(manifest.has_override("custom_window_management"));
    }

    fn manifest_with_override(extra: &str) -> Manifest {
        let contents = format!(
            r#"
[origin]
version = "0.1.0"
{extra}

[product]
id = "dev.origin.demo"
name = "Demo"
version = "0.1.0"

[security.windows.main]
profile = "standard-dashboard"
"#
        );
        toml::from_str(&contents).unwrap()
    }
}
