//! The MCP boundary (ADR-0027).
//!
//! MCP makes an application **controllable by an external AI**. It is not an inference
//! API — for inference the application performs itself, see `origin-ai`.
//!
//! ```text
//!   the user's own AI  ──MCP──▶  this application  ──▶  application services
//! ```
//!
//! The application therefore needs no model, no API key and no chat window of its own.
//! It states what it can do and what an AI is allowed to do with it.
//!
//! An MCP server is a **driving adapter**, the same role as the Tauri host or a CLI —
//! which is why nothing in this crate knows Tauri, and why a tool must be a service
//! method rather than a command.
//!
//! # The caller is a language model
//!
//! It acts on content it read somewhere, and that content may be hostile. A tool call
//! is therefore not a trusted request: the default grant is read and propose, never
//! commit or delete (see [`AiPermission`]).

mod permission;
mod protocol;
mod server;
mod tool;

pub use permission::{AiPermission, AiPermissions};
pub use protocol::{PROTOCOL_VERSION, Request, Response, ResponseError, ServerInfo};
pub use server::McpServer;
pub use tool::{Tool, ToolDescriptor, ToolOutput};
