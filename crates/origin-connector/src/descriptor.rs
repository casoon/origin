use origin_core::{ConnectorId, ProductPermission};
use serde::{Deserialize, Serialize};

/// How a connector authenticates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AuthKind {
    /// Authorization code flow with PKCE (ADR-0015).
    OAuth2,
    /// A token the user pastes in. Still stored in the credential store.
    PersonalAccessToken,
    /// Public data only.
    None,
}

/// What a connector is.
///
/// Kept separate from the trait so it can be rendered in a settings UI, serialised into
/// the app manifest, and reviewed without instantiating anything.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ConnectorDescriptor {
    pub id: ConnectorId,
    pub display_name: String,
    pub auth: AuthKind,

    /// The rights this connector needs at the external service.
    ///
    /// Declared, not inferred: a reviewer can see in one place whether an integration
    /// asks for write access, and a product can refuse to ship one that does.
    pub required_permissions: Vec<ProductPermission>,

    /// Whether the user may connect several accounts (ADR-0016).
    pub supports_multiple_accounts: bool,
}

impl ConnectorDescriptor {
    pub fn new(id: ConnectorId, display_name: impl Into<String>, auth: AuthKind) -> Self {
        Self {
            id,
            display_name: display_name.into(),
            auth,
            required_permissions: Vec::new(),
            supports_multiple_accounts: true,
        }
    }

    pub fn with_permissions(
        mut self,
        permissions: impl IntoIterator<Item = ProductPermission>,
    ) -> Self {
        self.required_permissions = permissions.into_iter().collect();
        self
    }

    pub fn single_account(mut self) -> Self {
        self.supports_multiple_accounts = false;
        self
    }

    /// Whether this connector asks for any write access.
    ///
    /// Products that want to stay read-only assert on this in a test.
    pub fn requests_write_access(&self) -> bool {
        self.required_permissions
            .iter()
            .any(ProductPermission::is_write)
    }
}

/// Who a set of credentials belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AccountIdentity {
    /// The service's own identifier — a GitHub login, a GA4 property id.
    pub external_id: String,
    /// What to show the user.
    pub display_name: String,
    /// Scopes the service reports as actually granted, which can be fewer than
    /// requested. Surfacing this is how a product explains a missing feature.
    pub granted_scopes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_read_only_connector_declares_no_write_permissions() {
        let descriptor =
            ConnectorDescriptor::new(ConnectorId::new("analytics"), "Analytics", AuthKind::OAuth2)
                .with_permissions([ProductPermission::read("analytics.reports")]);

        assert!(!descriptor.requests_write_access());
    }

    #[test]
    fn write_access_is_visible_in_the_descriptor() {
        let descriptor =
            ConnectorDescriptor::new(ConnectorId::new("github"), "GitHub", AuthKind::OAuth2)
                .with_permissions([
                    ProductPermission::read("notifications"),
                    ProductPermission::write("projects"),
                ]);

        assert!(descriptor.requests_write_access());
    }
}
