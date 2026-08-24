//! Account management (ADR-0016).
//!
//! An account is one authenticated identity at one connector. A connector may have
//! several; every cache key, sync record and credential is scoped by `AccountId` from
//! the start, because retrofitting that later touches everything.
//!
//! The account list lives in [`origin_storage::Storage`]; the credentials live in the
//! OS credential store (ADR-0008). Deleting the database therefore loses the list of
//! connected accounts, not the credentials — and a resync restores it.

mod service;
mod store;

pub use service::AccountService;
pub use store::AccountStore;

/// Storage namespace holding account records.
pub const ACCOUNTS_NAMESPACE: &str = "origin.accounts";
