//! Typed identifiers.
//!
//! Every id is a distinct type so that an `AccountId` can never be passed where a
//! `JobId` is expected. All of them serialise as plain strings.

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        // No `#[serde(transparent)]`: for a single-field tuple struct, serde_json
        // already serialises as the bare inner value, and ts-rs cannot parse the
        // attribute (it treats a one-field tuple struct as transparent on its own).
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        pub struct $name(String);

        impl $name {
            /// A fresh random id.
            pub fn generate() -> Self {
                Self(uuid::Uuid::new_v4().to_string())
            }

            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }
    };
}

string_id!(
    /// Identifies one configured account of one connector.
    AccountId
);
string_id!(
    /// Identifies a connector, e.g. `github`, `google-analytics`, `cloudflare`.
    ConnectorId
);
string_id!(
    /// Identifies an alert instance.
    AlertId
);
string_id!(
    /// Identifies a background job run.
    JobId
);
string_id!(
    /// Identifies one synchronisation run, used as a logging correlation id.
    SyncId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_serialise_as_plain_strings() {
        let id = AccountId::new("acc-1");
        assert_eq!(id.as_str(), "acc-1");
        assert_eq!(id.to_string(), "acc-1");
    }

    #[test]
    fn generated_ids_are_unique() {
        assert_ne!(JobId::generate(), JobId::generate());
    }
}
