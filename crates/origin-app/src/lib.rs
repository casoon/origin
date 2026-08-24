//! The composition root (ADR-0004).
//!
//! A product assembles its application here — and nowhere else. There is no global
//! state and no runtime service lookup by string: a missing dependency is a build
//! error, and the builder call itself documents the product's architecture.
//!
//! ```
//! # use origin_app::ApplicationBuilder;
//! let app = ApplicationBuilder::in_memory().build().unwrap();
//! assert!(app.modules().is_empty());
//! ```

mod application;
mod builder;
mod module;
mod platform;

pub use application::{AppInfo, Application};
pub use builder::{ApplicationBuilder, BuildError};
pub use module::{ApplicationModule, ModuleRegistry};
pub use platform::Platform;
