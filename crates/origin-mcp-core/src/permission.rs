use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// What an external AI may do with this application.
///
/// A third permission level, separate from product permissions (what the app may do at
/// a service) and platform permissions (what it may do on this machine). It is never
/// wider than the rights of the signed-in user: MCP must not be a privilege escalation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiPermission {
    /// Read a specific thing it already knows the identity of.
    Read,
    /// Search across content.
    Search,
    /// Prepare a change for a human to confirm. Nothing takes effect.
    Propose,
    /// Make a change take effect without further confirmation.
    Commit,
    /// Remove content.
    Delete,
}

impl AiPermission {
    /// Whether this permission can change anything.
    pub fn is_mutating(self) -> bool {
        matches!(self, Self::Commit | Self::Delete)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Search => "search",
            Self::Propose => "propose",
            Self::Commit => "commit",
            Self::Delete => "delete",
        }
    }
}

/// What this application actually grants.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AiPermissions {
    granted: BTreeSet<AiPermission>,
}

impl AiPermissions {
    /// Nothing granted. MCP is effectively off.
    pub fn none() -> Self {
        Self::default()
    }

    /// The default: an external AI may look at things and prepare changes, but nothing
    /// it does takes effect without a human.
    ///
    /// This is the whole safety story in one line. The caller is a model reacting to
    /// content it read somewhere; prompt injection in a document must not be able to
    /// delete anything.
    pub fn read_and_propose() -> Self {
        Self::from([
            AiPermission::Read,
            AiPermission::Search,
            AiPermission::Propose,
        ])
    }

    pub fn from(permissions: impl IntoIterator<Item = AiPermission>) -> Self {
        Self {
            granted: permissions.into_iter().collect(),
        }
    }

    pub fn allows(&self, permission: AiPermission) -> bool {
        self.granted.contains(&permission)
    }

    pub fn is_empty(&self) -> bool {
        self.granted.is_empty()
    }

    /// Whether anything granted can change data. Worth surfacing in the settings UI.
    pub fn grants_mutation(&self) -> bool {
        self.granted
            .iter()
            .any(|permission| permission.is_mutating())
    }

    pub fn granted(&self) -> impl Iterator<Item = AiPermission> + '_ {
        self.granted.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_grant_cannot_change_anything() {
        let permissions = AiPermissions::read_and_propose();

        assert!(permissions.allows(AiPermission::Read));
        assert!(permissions.allows(AiPermission::Propose));
        assert!(!permissions.allows(AiPermission::Commit));
        assert!(!permissions.allows(AiPermission::Delete));
        assert!(!permissions.grants_mutation());
    }

    #[test]
    fn proposing_is_not_mutating_but_committing_is() {
        assert!(!AiPermission::Propose.is_mutating());
        assert!(AiPermission::Commit.is_mutating());
        assert!(AiPermission::Delete.is_mutating());
    }

    #[test]
    fn granting_commit_is_visible_as_such() {
        let permissions = AiPermissions::from([AiPermission::Read, AiPermission::Commit]);

        assert!(
            permissions.grants_mutation(),
            "a settings screen must be able to warn about this"
        );
    }
}
