//! The `origin` command, as a library.
//!
//! Scaffolding is a tool concern, not a maintenance task a product carries: this crate
//! holds the project template and the code that instantiates it, while `origin-xtask`
//! keeps the tasks a generated project runs on itself (`validate`, `generate`,
//! `update`). That split is what lets `origin new` work without a checkout of Origin —
//! the template is compiled into this binary (ADR-0032).

pub mod scaffold;

mod template {
    include!(concat!(env!("OUT_DIR"), "/template.rs"));
}

pub use scaffold::{Options, run};

/// The embedded project template: `(path relative to the project root, contents)`.
///
/// A path may carry the `.in` suffix described in [`scaffold`]; it is stripped when the
/// file is written.
pub fn template_files() -> &'static [(&'static str, &'static [u8])] {
    template::FILES
}
