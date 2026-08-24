//! Typed event bus (ADR-0005).
//!
//! Events are Rust types. Subscribing is `bus.subscribe::<PlatformEvent>()`, not
//! `bus.on("platform:sync:completed")`. A renamed field is a compile error.
//!
//! Use an event when several independent components *may* react. If the caller needs
//! a result, call the service directly instead.

mod bus;
mod platform;

pub use bus::{Event, EventBus, EventStream, PublishError, RecvError, TryRecvError};
pub use platform::{
    AccountExpired, AlertRaised, AlertResolved, JobFinished, JobProgress, JobStarted,
    PlatformEvent, SyncCompleted, SyncFailed,
};
