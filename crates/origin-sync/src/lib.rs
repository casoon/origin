//! The sync engine.
//!
//! The division of labour is the whole point:
//!
//! ```text
//! A SyncSource decides HOW data is fetched.
//! The engine decides WHEN, and under which conditions.
//! ```
//!
//! Retry, exponential backoff, jitter, offline handling, validators and
//! single-flight belong to the platform. A connector that implements them itself
//! implements them differently from the next one, and none of them get tested.
//!
//! The engine is deliberately split so that scheduling is testable without waiting:
//! [`SyncEngine::run_due`] does one pass for a given instant, and the background loop
//! is a thin wrapper that calls it on a tick.

mod backoff;
mod engine;
mod health;
mod policy;
mod source;
mod state_store;
mod target;

pub use backoff::Backoff;
pub use engine::SyncEngine;
pub use health::{SyncStatus, health_of};
pub use policy::SyncPolicy;
pub use source::{SyncContext, SyncReport, SyncResult, SyncSource};
pub use target::SyncTarget;
