use crate::protocol::{PROTOCOL_VERSION, Request, Response, ServerInfo, codes};
use crate::{AiPermissions, Tool};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Serves an application's tools to an external AI client.
///
/// Transport-agnostic: it turns a request into a response and nothing more, so the same
/// server works over stdio, over a local HTTP endpoint, or over an in-memory pipe in a
/// test.
#[derive(Debug, Clone)]
pub struct McpServer {
    info: ServerInfo,
    tools: BTreeMap<String, Arc<dyn Tool>>,
    permissions: AiPermissions,
}

impl McpServer {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            info: ServerInfo {
                name: name.into(),
                version: version.into(),
            },
            tools: BTreeMap::new(),
            permissions: AiPermissions::none(),
        }
    }

    /// What an external AI is allowed to do. Nothing, unless the product says so.
    pub fn with_permissions(mut self, permissions: AiPermissions) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn with_tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.insert(tool.descriptor().name, tool);
        self
    }

    /// Tools the current grant actually permits.
    ///
    /// Tools beyond the grant are not merely refused on call — they are never listed.
    /// Advertising a tool that always fails wastes the model's attempts and teaches it
    /// to retry.
    fn available(&self) -> impl Iterator<Item = &Arc<dyn Tool>> {
        self.tools
            .values()
            .filter(|tool| self.permissions.allows(tool.descriptor().permission))
    }

    /// Handle one request. Returns `None` for a notification.
    pub async fn handle(&self, request: Request) -> Option<Response> {
        let Some(id) = request.id.clone() else {
            tracing::debug!(method = %request.method, "mcp notification");
            return None;
        };

        let response = match request.method.as_str() {
            "initialize" => Response::result(id, self.initialize()),
            "tools/list" => Response::result(id, self.list_tools()),
            "tools/call" => self.call_tool(id, request.params).await,
            "ping" => Response::result(id, json!({})),
            other => Response::error(
                id,
                codes::METHOD_NOT_FOUND,
                format!("unsupported method `{other}`"),
            ),
        };

        Some(response)
    }

    fn initialize(&self) -> Value {
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "serverInfo": self.info,
            "capabilities": { "tools": {} }
        })
    }

    fn list_tools(&self) -> Value {
        let tools: Vec<Value> = self
            .available()
            .map(|tool| {
                let descriptor = tool.descriptor();
                json!({
                    "name": descriptor.name,
                    "title": descriptor.title,
                    "description": descriptor.description,
                    "inputSchema": descriptor.input_schema,
                })
            })
            .collect();

        json!({ "tools": tools })
    }

    async fn call_tool(&self, id: Value, params: Value) -> Response {
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return Response::error(id, codes::INVALID_PARAMS, "missing tool name");
        };

        let Some(tool) = self.tools.get(name) else {
            return Response::error(id, codes::INVALID_PARAMS, format!("unknown tool `{name}`"));
        };

        let descriptor = tool.descriptor();
        if !self.permissions.allows(descriptor.permission) {
            // Refused at the boundary, and recorded: an external agent repeatedly
            // reaching for a permission it does not have is worth seeing in a log.
            tracing::warn!(
                tool = name,
                permission = descriptor.permission.as_str(),
                "mcp tool call refused: permission not granted"
            );
            return Response::error(
                id,
                codes::INVALID_REQUEST,
                format!(
                    "`{name}` needs the `{}` permission, which this application does not grant",
                    descriptor.permission.as_str()
                ),
            );
        }

        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        tracing::info!(
            tool = name,
            permission = descriptor.permission.as_str(),
            "mcp tool call"
        );

        match tool.call(arguments).await {
            Ok(output) => {
                let mut result = json!({
                    "content": [{ "type": "text", "text": output.text }],
                    "isError": false,
                });
                if let Some(structured) = output.structured {
                    result["structuredContent"] = structured;
                }
                Response::result(id, result)
            }

            // A failed tool is reported to the model as a tool error, not as a
            // protocol error: the model can read it and choose differently, which a
            // transport-level failure does not allow.
            Err(error) => {
                tracing::warn!(tool = name, %error, "mcp tool call failed");
                Response::result(
                    id,
                    json!({
                        "content": [{
                            "type": "text",
                            "text": error.to_contract().message,
                        }],
                        "isError": true,
                    }),
                )
            }
        }
    }

    /// Parse and handle one line of JSON. Convenience for line-based transports.
    pub async fn handle_line(&self, line: &str) -> Option<Response> {
        match serde_json::from_str::<Request>(line) {
            Ok(request) => self.handle(request).await,
            Err(error) => Some(Response::error(
                Value::Null,
                codes::INTERNAL_ERROR,
                format!("malformed request: {error}"),
            )),
        }
    }
}
