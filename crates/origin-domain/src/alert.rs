//! Alerts are a universal concept: a product decides what raises one, the platform
//! decides how it is deduplicated, surfaced and resolved.

use crate::ids::{AccountId, AlertId, ConnectorId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AlertState {
    Active,
    Acknowledged,
    Resolved,
    /// Suppressed by the user; still tracked, but never surfaced.
    Silenced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Alert {
    pub id: AlertId,
    /// Stable identity of *the problem*, not of this occurrence. Two raises with the
    /// same fingerprint are the same alert, so the user is not notified twice.
    pub fingerprint: String,
    pub severity: Severity,
    pub title: String,
    pub body: Option<String>,
    pub connector: Option<ConnectorId>,
    pub account: Option<AccountId>,
    pub state: AlertState,
    #[serde(with = "time::serde::rfc3339")]
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub raised_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    #[cfg_attr(feature = "ts", ts(type = "string | null"))]
    pub resolved_at: Option<OffsetDateTime>,
}

impl Alert {
    pub fn new(
        fingerprint: impl Into<String>,
        severity: Severity,
        title: impl Into<String>,
        raised_at: OffsetDateTime,
    ) -> Self {
        Self {
            id: AlertId::generate(),
            fingerprint: fingerprint.into(),
            severity,
            title: title.into(),
            body: None,
            connector: None,
            account: None,
            state: AlertState::Active,
            raised_at,
            resolved_at: None,
        }
    }

    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn with_connector(mut self, connector: ConnectorId) -> Self {
        self.connector = Some(connector);
        self
    }

    /// Whether this alert should currently be shown to the user.
    pub fn is_visible(&self) -> bool {
        matches!(self.state, AlertState::Active | AlertState::Acknowledged)
    }

    pub fn resolve(&mut self, at: OffsetDateTime) {
        self.state = AlertState::Resolved;
        self.resolved_at = Some(at);
    }
}
