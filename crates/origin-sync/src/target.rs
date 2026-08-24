use origin_core::{AccountId, ConnectorId};
use origin_storage::namespace;
use serde::{Deserialize, Serialize};
use std::fmt;

/// One thing that gets synchronised: a kind of data, for one account, at one connector.
///
/// `name` is the product's own label — `notifications`, `projects`, `traffic`. Every
/// target is account-scoped (ADR-0016), which is also what lets its state disappear
/// with the account.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SyncTarget {
    pub connector: ConnectorId,
    pub account: AccountId,
    pub name: String,
}

impl SyncTarget {
    pub fn new(connector: ConnectorId, account: AccountId, name: impl Into<String>) -> Self {
        Self {
            connector,
            account,
            name: name.into(),
        }
    }

    /// Storage namespace holding this target's sync state.
    ///
    /// Under the account prefix, so disconnecting the account removes it (ADR-0019).
    pub(crate) fn namespace(&self) -> String {
        namespace::account(&self.connector, &self.account, "sync")
    }
}

impl fmt::Display for SyncTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}/{}", self.connector, self.account, self.name)
    }
}
