//! How a product is released (ADR-0030).
//!
//! Everything here is *declaration*. Signing identities and notarisation credentials
//! are never in the manifest — they are CI secrets, and a product that has none builds
//! unsigned artifacts and says so.

use serde::{Deserialize, Serialize};

/// Which audience a build is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Channel {
    #[default]
    Stable,
    Beta,
    Nightly,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Nightly => "nightly",
        }
    }
}

/// Platforms a release builds for.
///
/// The names are written out rather than derived: `rename_all` turns `MacosX86_64` into
/// `macos-x86-64`, which is not what anyone writes in a manifest and not what
/// [`Target::as_str`] returns. Two spellings for one target is a bug waiting for a
/// release day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Target {
    #[serde(rename = "macos-aarch64")]
    MacosAarch64,
    #[serde(rename = "macos-x86_64")]
    MacosX86_64,
    #[serde(rename = "windows-x86_64")]
    WindowsX86_64,
    #[serde(rename = "linux-x86_64")]
    LinuxX86_64,
}

impl Target {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MacosAarch64 => "macos-aarch64",
            Self::MacosX86_64 => "macos-x86_64",
            Self::WindowsX86_64 => "windows-x86_64",
            Self::LinuxX86_64 => "linux-x86_64",
        }
    }

    /// Whether this target needs code signing to be installable without warnings.
    pub fn needs_signing(self) -> bool {
        !matches!(self, Self::LinuxX86_64)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DistributionSection {
    pub channel: Channel,
    pub targets: Vec<Target>,
    pub updater: UpdaterSection,
}

impl Default for DistributionSection {
    fn default() -> Self {
        Self {
            channel: Channel::default(),
            targets: vec![
                Target::MacosAarch64,
                Target::MacosX86_64,
                Target::WindowsX86_64,
                Target::LinuxX86_64,
            ],
            updater: UpdaterSection::default(),
        }
    }
}

/// In-app updates.
///
/// Off by default, and deliberately so: an updater that cannot verify a signature is
/// worse than no updater, because it turns a compromised endpoint into arbitrary code
/// execution on every installation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdaterSection {
    pub enabled: bool,
    /// Where the update manifest is published. One per channel is the usual shape.
    pub endpoints: Vec<String>,
}

impl DistributionSection {
    /// Targets that would ship unsigned unless CI has an identity for them.
    pub fn targets_needing_signing(&self) -> Vec<Target> {
        self.targets
            .iter()
            .copied()
            .filter(|target| target.needs_signing())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_updater_is_off_until_someone_turns_it_on() {
        let distribution = DistributionSection::default();

        assert!(!distribution.updater.enabled);
        assert!(distribution.updater.endpoints.is_empty());
    }

    #[test]
    fn linux_is_the_one_target_that_ships_without_signing() {
        let distribution = DistributionSection::default();

        let needing = distribution.targets_needing_signing();

        assert_eq!(needing.len(), 3);
        assert!(!needing.contains(&Target::LinuxX86_64));
    }

    /// Guards the mismatch this file's comment describes: the serialised name and the
    /// displayed name must be the same string, for every target.
    #[test]
    fn every_target_serialises_exactly_as_it_displays() {
        for target in [
            Target::MacosAarch64,
            Target::MacosX86_64,
            Target::WindowsX86_64,
            Target::LinuxX86_64,
        ] {
            let serialised = serde_json::to_string(&target).expect("serialise");
            assert_eq!(
                serialised.trim_matches('"'),
                target.as_str(),
                "a manifest written with `{}` must parse",
                target.as_str()
            );

            let parsed: Target = serde_json::from_str(&serialised).expect("round trip");
            assert_eq!(parsed, target);
        }
    }

    #[test]
    fn a_channel_round_trips_through_the_manifest_format() {
        let parsed: Channel = toml::from_str::<toml::Table>("value = \"beta\"")
            .map(|table| table["value"].clone().try_into().unwrap())
            .unwrap();

        assert_eq!(parsed, Channel::Beta);
        assert_eq!(parsed.as_str(), "beta");
    }
}
