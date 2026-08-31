//! The storage namespace convention (ADR-0019).
//!
//! Every record lives in a namespace, and the namespace decides who owns the data and
//! when it may be removed. Three shapes, no exceptions:
//!
//! ```text
//! origin.<area>                      platform data, not tied to an account
//!                                    origin.settings · origin.accounts
//!
//! acct.<connector>.<account>.<area>  anything belonging to one connected account
//!                                    acct.github.a1b2.notifications · …sync
//!
//! app.<module>.<area>                product data with no account behind it
//!                                    app.planning.templates
//! ```
//!
//! The account prefix is what makes disconnecting an account a mechanical operation:
//! [`Storage::clear_prefix`] removes everything below it, without any module having to
//! declare which namespaces it wrote.
//!
//! [`Storage::clear_prefix`]: crate::Storage::clear_prefix

use origin_domain::{AccountId, ConnectorId};

/// Namespace for platform-owned data.
pub fn platform(area: &str) -> String {
    format!("origin.{area}")
}

/// Namespace for data belonging to one account of one connector.
pub fn account(connector: &ConnectorId, account: &AccountId, area: &str) -> String {
    format!("acct.{connector}.{account}.{area}")
}

/// Prefix covering *every* namespace of one account.
///
/// Ends with a separator so that account `a1` cannot match account `a1b2`.
pub fn account_prefix(connector: &ConnectorId, account: &AccountId) -> String {
    format!("acct.{connector}.{account}.")
}

/// Namespace for product data that is not tied to an account.
pub fn module(module: &str, area: &str) -> String {
    format!("app.{module}.{area}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_account_prefix_covers_its_namespaces() {
        let connector = ConnectorId::new("github");
        let id = AccountId::new("a1b2");

        let prefix = account_prefix(&connector, &id);

        assert!(account(&connector, &id, "notifications").starts_with(&prefix));
        assert!(account(&connector, &id, "sync").starts_with(&prefix));
    }

    #[test]
    fn a_prefix_cannot_match_a_longer_account_id() {
        let connector = ConnectorId::new("github");

        let prefix = account_prefix(&connector, &AccountId::new("a1"));
        let other = account(&connector, &AccountId::new("a1b2"), "sync");

        assert!(
            !other.starts_with(&prefix),
            "the trailing separator is what prevents this: {prefix} vs {other}"
        );
    }

    #[test]
    fn platform_and_product_namespaces_never_collide_with_account_data() {
        assert!(!platform("settings").starts_with("acct."));
        assert!(!module("planning", "templates").starts_with("acct."));
    }
}
