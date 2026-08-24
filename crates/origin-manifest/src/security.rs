use crate::{Manifest, SecurityProfile};
use serde::Serialize;
use std::collections::BTreeMap;

/// A Tauri capability file, as generated from the manifest.
#[derive(Debug, Clone, Serialize)]
pub struct Capability {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub identifier: String,
    pub description: String,
    pub windows: Vec<String>,
    pub permissions: Vec<String>,
}

impl Capability {
    /// One capability file per profile, listing the windows that use it.
    ///
    /// Grouped by profile rather than per window: two windows with the same profile
    /// should visibly share one grant, so widening it is one obvious diff instead of
    /// two that can drift apart.
    pub fn from_manifest(manifest: &Manifest) -> Vec<Self> {
        let mut by_profile: BTreeMap<SecurityProfile, Vec<String>> = BTreeMap::new();

        for (window, security) in &manifest.security.windows {
            by_profile
                .entry(security.profile)
                .or_default()
                .push(window.clone());
        }

        by_profile
            .into_iter()
            .map(|(profile, mut windows)| {
                windows.sort();
                Self {
                    schema: "../gen/schemas/desktop-schema.json".to_owned(),
                    identifier: profile.identifier().to_owned(),
                    description: format!(
                        "{} Generated from app.toml — do not edit.",
                        profile.description()
                    ),
                    windows,
                    permissions: profile
                        .permissions()
                        .iter()
                        .map(|permission| (*permission).to_owned())
                        .collect(),
                }
            })
            .collect()
    }

    /// File name this capability is written to.
    pub fn file_name(&self) -> String {
        format!("{}.json", self.identifier)
    }
}

// `SecurityProfile` is used as a map key above, so it needs a total order.
impl PartialOrd for SecurityProfile {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SecurityProfile {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.identifier().cmp(other.identifier())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(windows: &str) -> Manifest {
        let contents = format!(
            r#"
[origin]
version = "0.1.0"

[product]
id = "dev.origin.demo"
name = "Demo"
version = "0.1.0"
{windows}
"#
        );
        toml::from_str(&contents).unwrap()
    }

    #[test]
    fn windows_sharing_a_profile_share_one_capability_file() {
        let manifest = manifest(
            "\n[security.windows.main]\nprofile = \"standard-dashboard\"\n\
             \n[security.windows.detail]\nprofile = \"standard-dashboard\"\n",
        );

        let capabilities = Capability::from_manifest(&manifest);

        assert_eq!(capabilities.len(), 1);
        assert_eq!(capabilities[0].windows, vec!["detail", "main"]);
    }

    #[test]
    fn different_profiles_produce_separate_files() {
        let manifest = manifest(
            "\n[security.windows.main]\nprofile = \"standard-dashboard\"\n\
             \n[security.windows.settings]\nprofile = \"account-settings\"\n",
        );

        let capabilities = Capability::from_manifest(&manifest);

        assert_eq!(capabilities.len(), 2);
        assert_eq!(capabilities[0].file_name(), "account-settings.json");
        assert_eq!(capabilities[1].file_name(), "standard-dashboard.json");
    }

    #[test]
    fn the_generated_file_says_it_is_generated() {
        let manifest = manifest("\n[security.windows.main]\nprofile = \"readonly-dashboard\"\n");

        let capability = &Capability::from_manifest(&manifest)[0];

        assert!(
            capability.description.contains("do not edit"),
            "someone opening this file must see where it comes from"
        );
    }
}
