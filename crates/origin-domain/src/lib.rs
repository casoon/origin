//! Domain primitives and ports shared by every Origin-based application.
//!
//! This crate is the bottom of the dependency graph. It must never depend on
//! Tauri, on a storage engine, on an HTTP client, or on a product.

pub mod account;
pub mod alert;
pub mod clock;
pub mod error;
pub mod health;
pub mod ids;
pub mod job;
pub mod metric;
pub mod permission;
pub mod sync;

#[cfg(feature = "testing")]
pub mod testing;

pub use account::{Account, AccountStatus};
pub use alert::{Alert, AlertState, Severity};
pub use clock::{Clock, SystemClock};
pub use error::{AppError, ErrorContract, ErrorKind, Result};
pub use health::Health;
pub use ids::{AccountId, AlertId, ConnectorId, JobId, SyncId};
pub use job::{Job, JobStatus, Progress};
pub use metric::{Metric, MetricKey, Trend, Unit};
pub use permission::{PlatformPermission, ProductPermission};
pub use sync::{SyncOutcome, SyncState};
