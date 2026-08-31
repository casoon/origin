//! The connector contract (ADR-0006).
//!
//! A connector declares *what* it is and how to reach it. The platform decides *when*
//! to talk to it. It is deliberately small: scheduling and sync targets arrive in
//! Phase 3, once there is a second connector to validate the shape against (ADR-0009).

mod descriptor;
mod registry;

pub use descriptor::{AccountIdentity, AuthKind, ConnectorDescriptor};
pub use registry::ConnectorRegistry;

use async_trait::async_trait;
use origin_domain::{AccountId, ConnectorId, Result};
use std::fmt::Debug;

/// An integration with one external service.
#[async_trait]
pub trait Connector: Debug + Send + Sync + 'static {
    fn id(&self) -> ConnectorId;

    /// What this connector is and what it needs. Used by the settings UI and by the
    /// permission review — a connector cannot quietly widen its scopes.
    fn descriptor(&self) -> ConnectorDescriptor;

    /// Prove that an account's credentials still work, and report who they belong to.
    ///
    /// This is the one operation every connector must support. It is what turns
    /// "we have a token" into "we have a working account", and it runs after
    /// authorization and whenever credentials are suspected to be stale.
    ///
    /// Returns `AppError::Authentication` when the credentials are no longer valid —
    /// the platform reacts by marking the account expired, not by retrying.
    async fn verify(&self, account: &AccountId) -> Result<AccountIdentity>;
}
