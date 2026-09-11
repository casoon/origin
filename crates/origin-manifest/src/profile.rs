use serde::{Deserialize, Serialize};

/// A named security profile (ADR-0007, §20).
///
/// Profiles exist so that granting a window its permissions is a *decision between
/// named options*, not a free-form list somebody copies from another project and
/// widens by one line at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SecurityProfile {
    /// Reads application state and receives events. Nothing else.
    ReadonlyDashboard,

    /// The default for a main window: state, events, and clipboard for copyable data.
    StandardDashboard,

    /// Manages accounts and credentials. Credential handling itself happens in Rust —
    /// this profile does not grant the frontend access to secrets.
    AccountSettings,

    /// A workspace window that may read files under user-confirmed roots and execute
    /// programs listed in the process allowlist.
    ///
    /// This is the narrowest useful grant for a window that touches the local file
    /// system — explicitly *not* `fs:default` / `shell:default`, and every permission
    /// maps to a platform contract (ADR-0007).
    LocalWorkspace,
}

impl SecurityProfile {
    /// The Tauri permissions this profile grants.
    ///
    /// Listed explicitly rather than pulling in a plugin's `default` set: a plugin
    /// default grows when the plugin is updated, silently widening every window that
    /// used it.
    pub fn permissions(self) -> &'static [&'static str] {
        match self {
            Self::ReadonlyDashboard => &[
                "core:default",
                "core:event:allow-listen",
                "core:event:allow-unlisten",
            ],
            Self::StandardDashboard => &[
                "core:default",
                "core:event:allow-listen",
                "core:event:allow-unlisten",
            ],
            Self::AccountSettings => &[
                "core:default",
                "core:event:allow-listen",
                "core:event:allow-unlisten",
                "core:window:allow-close",
            ],
            Self::LocalWorkspace => &[
                "core:default",
                "core:event:allow-listen",
                "core:event:allow-unlisten",
            ],
        }
    }

    pub fn identifier(self) -> &'static str {
        match self {
            Self::ReadonlyDashboard => "readonly-dashboard",
            Self::StandardDashboard => "standard-dashboard",
            Self::AccountSettings => "account-settings",
            Self::LocalWorkspace => "local-workspace",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::ReadonlyDashboard => {
                "Reads application state and receives platform events. No filesystem, \
                 no shell, no process execution."
            }
            Self::StandardDashboard => {
                "Main window: reads application state and receives platform events. No \
                 filesystem, no shell, no process execution."
            }
            Self::AccountSettings => {
                "Settings window: manages accounts through commands. Credentials never \
                 reach the frontend."
            }
            Self::LocalWorkspace => {
                "Workspace window: access to workspace files and allowlisted processes \
                 via Origin commands. No direct Tauri fs or shell plugin access."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_profile_grants_fs_shell_or_process() {
        let profiles = [
            SecurityProfile::ReadonlyDashboard,
            SecurityProfile::StandardDashboard,
            SecurityProfile::AccountSettings,
            SecurityProfile::LocalWorkspace,
        ];

        for profile in profiles {
            for permission in profile.permissions() {
                assert!(
                    !permission.starts_with("fs:")
                        && !permission.starts_with("shell:")
                        && !permission.starts_with("process:"),
                    "{} grants {permission}",
                    profile.identifier()
                );
            }
        }
    }

    #[test]
    fn all_profiles_have_unique_identifiers() {
        let profiles = [
            SecurityProfile::ReadonlyDashboard,
            SecurityProfile::StandardDashboard,
            SecurityProfile::AccountSettings,
            SecurityProfile::LocalWorkspace,
        ];

        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for profile in profiles {
            let id = profile.identifier();
            assert!(seen.insert(id), "duplicate identifier: {id}");
            assert!(
                !profile.description().is_empty(),
                "{id}: description must not be empty"
            );
        }
    }

    #[test]
    fn local_workspace_round_trips_through_the_manifest_format() {
        let parsed: SecurityProfile = toml::from_str("value = \"local-workspace\"")
            .map(|table: toml::Table| table["value"].clone())
            .map(|value| value.try_into().unwrap())
            .unwrap();

        assert_eq!(parsed, SecurityProfile::LocalWorkspace);
        assert_eq!(parsed.identifier(), "local-workspace");
    }

    #[test]
    fn profiles_round_trip_through_the_manifest_format() {
        let parsed: SecurityProfile = toml::from_str("value = \"account-settings\"")
            .map(|table: toml::Table| table["value"].clone())
            .map(|value| value.try_into().unwrap())
            .unwrap();

        assert_eq!(parsed, SecurityProfile::AccountSettings);
        assert_eq!(parsed.identifier(), "account-settings");
    }
}
