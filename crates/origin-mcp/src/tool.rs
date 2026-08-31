use crate::AiPermission;
use async_trait::async_trait;
use origin_domain::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// What a tool is, as the model sees it.
///
/// The description is the part a model actually acts on — it is public API for a
/// reader that cannot ask follow-up questions. It belongs in review like any other
/// interface, not appended as an afterthought.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescriptor {
    /// Namespaced, dot-separated: `projects.list`, `knowledge.search`.
    pub name: String,
    /// Short human-readable label for permission dialogs and logs.
    pub title: String,
    /// What the tool does, when to use it, and what it returns.
    pub description: String,
    /// What invoking this costs in terms of rights.
    pub permission: AiPermission,
    /// JSON Schema for the arguments.
    pub input_schema: serde_json::Value,
}

impl ToolDescriptor {
    pub fn new(
        name: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        permission: AiPermission,
    ) -> Self {
        Self {
            name: name.into(),
            title: title.into(),
            description: description.into(),
            permission,
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        }
    }

    pub fn with_schema(mut self, input_schema: serde_json::Value) -> Self {
        self.input_schema = input_schema;
        self
    }
}

/// What a tool returns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Rendered for the model to read.
    pub text: String,
    /// The same answer as data, when the caller can use it.
    pub structured: Option<serde_json::Value>,
}

impl ToolOutput {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            structured: None,
        }
    }

    pub fn with_structured(mut self, structured: serde_json::Value) -> Self {
        self.structured = Some(structured);
        self
    }
}

/// One operation an external AI may invoke.
///
/// A tool wraps an *application service*, never a Tauri command: an operation that
/// exists only as a command is not reachable from MCP, from a CLI or from a headless
/// run. If writing a tool means duplicating logic, the logic is in the wrong place.
#[async_trait]
pub trait Tool: Debug + Send + Sync + 'static {
    fn descriptor(&self) -> ToolDescriptor;

    /// Arguments arrive as JSON, validated by the caller against nothing in
    /// particular — check them.
    async fn call(&self, arguments: serde_json::Value) -> Result<ToolOutput>;
}
