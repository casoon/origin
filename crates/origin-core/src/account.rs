//! An account is one authenticated identity at one connector.

use crate::ids::{AccountId, ConnectorId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    Active,
    /// Credentials exist but are no longer valid — the user must re-authenticate.
    Expired,
    /// The user disconnected the account; credentials have been removed.
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Account {
    pub id: AccountId,
    pub connector: ConnectorId,
    /// What the user sees. Never a token, never an internal handle.
    pub display_name: String,
    pub status: AccountStatus,
    #[serde(with = "time::serde::rfc3339")]
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub connected_at: OffsetDateTime,
}

impl Account {
    pub fn is_usable(&self) -> bool {
        matches!(self.status, AccountStatus::Active)
    }
}
