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
}

impl SecurityProfile {
    /// The Tauri permissions this profile grants.
    ///
    /// Listed explicitly rather than pulling in a plugin's `default` set: a plugin
    /// default grows when the plugin is updated, silently widening every window that
    /// used it.
    ///
    /// Note what no profile grants: filesystem, shell or process access. Opening URLs
    /// and showing notifications happen in Rust behind platform contracts, so the
    /// frontend needs no permission for either.
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
        }
    }

    pub fn identifier(self) -> &'static str {
        match self {
            Self::ReadonlyDashboard => "readonly-dashboard",
            Self::StandardDashboard => "standard-dashboard",
            Self::AccountSettings => "account-settings",
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_profile_grants_filesystem_shell_or_process_access() {
        for profile in [
            SecurityProfile::ReadonlyDashboard,
            SecurityProfile::StandardDashboard,
            SecurityProfile::AccountSettings,
        ] {
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
    fn profiles_round_trip_through_the_manifest_format() {
        let parsed: SecurityProfile = toml::from_str("value = \"account-settings\"")
            .map(|table: toml::Table| table["value"].clone())
            .map(|value| value.try_into().unwrap())
            .unwrap();

        assert_eq!(parsed, SecurityProfile::AccountSettings);
        assert_eq!(parsed.identifier(), "account-settings");
    }
}
