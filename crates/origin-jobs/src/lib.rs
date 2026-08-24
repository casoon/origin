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

mod context;
mod registry;

pub use context::JobContext;
pub use registry::Jobs;
