//! Local storage: cache, read models, sync metadata, settings (ADR-0008).
//!
//! External services stay Source of Truth. Deleting the local store must never lose
//! user data.
//!
//! [`Storage`] is deliberately dumb persistence — it stores and returns records as
//! given. Expiry is enforced one layer up, by [`Cache`], so that every backend behaves
//! identically regardless of what it can express natively.

mod cache;
mod memory;
pub mod namespace;
mod store;

#[cfg(feature = "testing")]
pub mod contract;

pub use cache::Cache;
pub use memory::MemoryStorage;
pub use store::{Record, Storage, StorageKey};
