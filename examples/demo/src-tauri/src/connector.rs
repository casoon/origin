//! A connector with no external service behind it.
//!
//! Real connectors talk to GitHub, Google or Cloudflare. This one exists to show the
//! shape: what a connector declares, and what the platform does with that declaration.
//! It needs no credentials, which is what keeps the demo runnable for anyone.

use async_trait::async_trait;
use origin_connector::{AccountIdentity, AuthKind, Connector, ConnectorDescriptor};
use origin_domain::{AccountId, ConnectorId, ProductPermission, Result};

#[derive(Debug, Clone, Copy)]
pub struct DemoConnector;

impl DemoConnector {
    pub fn id() -> ConnectorId {
        ConnectorId::new("demo")
    }
}

#[async_trait]
impl Connector for DemoConnector {
    fn id(&self) -> ConnectorId {
        Self::id()
    }

    fn descriptor(&self) -> ConnectorDescriptor {
        ConnectorDescriptor::new(Self::id(), "Origin Demo Service", AuthKind::None)
            // Declared read-only. A reviewer can see that in one place, and the test
            // below fails if someone quietly adds write access.
            .with_permissions([ProductPermission::read("demo.load")])
    }

    async fn verify(&self, account: &AccountId) -> Result<AccountIdentity> {
        Ok(AccountIdentity {
            external_id: account.to_string(),
            display_name: "Local demo account".to_owned(),
            granted_scopes: vec!["demo.load".to_owned()],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_demo_connector_stays_read_only() {
        assert!(!DemoConnector.descriptor().requests_write_access());
    }
}
