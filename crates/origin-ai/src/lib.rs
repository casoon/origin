//! Inference the application performs itself.
//!
//! This is the **embedded** half of Origin's AI story, and it has nothing to do with
//! MCP. The two are deliberately separate (ADR-0027):
//!
//! ```text
//!                          ┌── MCP server   external AI controls the app
//! UI ── Application Core ──┤
//!                          └── AIService    the app performs inference itself
//! ```
//!
//! Using MCP as a general inference API would be the wrong abstraction: the protocol
//! connects a client to a server's tools, not an application to somebody's model
//! subscription.
//!
//! Domain code depends on [`AiService`], never on a provider. A product without the
//! capability simply has no service, and every AI feature it has is off — which is the
//! point: nothing an Origin application does may *depend* on a model being reachable.

mod request;
mod service;

#[cfg(feature = "testing")]
pub mod testing;

pub use request::{Completion, Prompt, Usage};
pub use service::{AiService, NoopAiService};
