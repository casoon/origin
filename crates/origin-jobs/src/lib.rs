//! Background jobs.
//!
//! Everything long-running — a repository scan, a large export, a report render —
//! reports progress the same way, so every product reuses one progress UI and one
//! cancel button.
//!
//! ```ignore
//! let id = jobs.spawn("export", |ctx| async move {
//!     for (index, item) in items.iter().enumerate() {
//!         if ctx.is_cancelled() {
//!             return Ok(());
//!         }
//!         ctx.progress(index as u64 + 1, Some(items.len() as u64)).await;
//!         write(item).await?;
//!     }
//!     Ok(())
//! });
//! ```
//!
//! Fire-and-forget covers most jobs, but two things `spawn` alone cannot do:
//!
//! - **Only one at a time.** `spawn_exclusive` refuses to start a second job of the
//!   same `kind` while one is still running, instead of silently letting both proceed.
//! - **Give the caller its result.** `Job` (what [`Jobs::get`]/[`Jobs::list`] return) is
//!   deliberately kind-agnostic and IPC-safe, with no slot for a typed value. A caller
//!   that needs what the job actually produced — not just that it finished — uses
//!   `spawn_awaitable` and awaits the returned [`JobResult`].
//!
//! `spawn_exclusive_awaitable` is both at once: the common shape for "run this, only
//! one at a time, and hand me back what it computed" — a flow whose caller already
//! awaits synchronously (a request/response command handler) rather than polling
//! [`Jobs::get`] or subscribing to progress events.
//!
//! ```ignore
//! let (_id, result) = jobs.spawn_exclusive_awaitable("crawl", |ctx| async move {
//!     let report = crawl(&ctx).await?;
//!     Ok(report)
//! })?;
//! let report = result.wait().await?;
//! ```

mod context;
mod registry;

pub use context::JobContext;
pub use registry::{JobResult, Jobs};
