use crate::protocol::{InitializeParams, PROTOCOL_VERSION, Request, Response, ServerInfo, codes};
use crate::{AiPermissions, Tool};
use origin_platform::{ConfirmationDecision, ConfirmationRequest, ConfirmationService};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Process-wide counter so every session gets a distinct, greppable id.
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_session_id() -> String {
    let number = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("mcp-session-{number}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lifecycle {
    Uninitialized,
    AwaitingInitialized,
    Ready,
}

/// Serves an application's tools to an external AI client.
///
/// Transport-agnostic: it turns a request into a response and nothing more, so the same
/// server works over stdio, over a local HTTP endpoint, or over an in-memory pipe in a
/// test.
#[derive(Debug)]
pub struct McpServer {
    info: ServerInfo,
    /// Identifies one session in logs. A clone starts a new session, so a stdio
    /// connection and an HTTP client are distinguishable in the trace.
    session_id: String,
    tools: BTreeMap<String, Arc<dyn Tool>>,
    permissions: AiPermissions,
    /// Required before a mutating tool runs. `None` means block (fail-closed),
    /// so a product that never wires a confirmer cannot grant mutation through
    /// the AI boundary.
    confirmation: Option<Arc<dyn ConfirmationService>>,
    lifecycle: Mutex<Lifecycle>,
}

impl Clone for McpServer {
    fn clone(&self) -> Self {
        Self {
            info: self.info.clone(),
            session_id: next_session_id(),
            tools: self.tools.clone(),
            permissions: self.permissions.clone(),
            confirmation: self.confirmation.clone(),
            // A clone represents another transport session. Tools are shared, but MCP
            // lifecycle state is connection-local.
            lifecycle: Mutex::new(Lifecycle::Uninitialized),
        }
    }
}

impl McpServer {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            info: ServerInfo {
                name: name.into(),
                version: version.into(),
            },
            session_id: next_session_id(),
            tools: BTreeMap::new(),
            permissions: AiPermissions::none(),
            confirmation: None,
            lifecycle: Mutex::new(Lifecycle::Uninitialized),
        }
    }

    /// What an external AI is allowed to do. Nothing, unless the product says so.
    pub fn with_permissions(mut self, permissions: AiPermissions) -> Self {
        self.permissions = permissions;
        self
    }

    /// Ask a human before a mutating tool takes effect.
    ///
    /// Without this, `Commit` and `Delete` tools are refused at the boundary — the
    /// safest default. A product that grants mutation must wire a confirmer (a native
    /// dialog, a policy file) at composition time.
    pub fn with_confirmation(mut self, confirmation: Arc<dyn ConfirmationService>) -> Self {
        self.confirmation = Some(confirmation);
        self
    }

    /// The id this session is logged under.
    pub fn session_id(&self) -> &str {
        &self.session_id
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
        if request.jsonrpc != "2.0" {
            return Some(Response::error(
                request.id.unwrap_or(Value::Null),
                codes::INVALID_REQUEST,
                "jsonrpc must be `2.0`",
            ));
        }

        let Some(id) = request.id.clone() else {
            if request.method == "notifications/initialized" {
                let mut lifecycle = self.lifecycle();
                if *lifecycle == Lifecycle::AwaitingInitialized {
                    *lifecycle = Lifecycle::Ready;
                } else {
                    tracing::warn!("unexpected MCP initialized notification");
                }
            }
            tracing::debug!(method = %request.method, "mcp notification");
            return None;
        };

        let response = match request.method.as_str() {
            "initialize" => self.initialize(id, request.params),
            "ping" => Response::result(id, json!({})),
            _ if *self.lifecycle() != Lifecycle::Ready => {
                Response::error(id, codes::INVALID_REQUEST, "MCP session is not initialized")
            }
            "tools/list" => Response::result(id, self.list_tools()),
            "tools/call" => self.call_tool(id, request.params).await,
            other => Response::error(
                id,
                codes::METHOD_NOT_FOUND,
                format!("unsupported method `{other}`"),
            ),
        };

        Some(response)
    }

    fn initialize(&self, id: Value, params: Value) -> Response {
        let params: InitializeParams = match serde_json::from_value(params) {
            Ok(params) => params,
            Err(error) => {
                return Response::error(
                    id,
                    codes::INVALID_PARAMS,
                    format!("invalid initialize parameters: {error}"),
                );
            }
        };
        if !params.capabilities.is_object() {
            return Response::error(
                id,
                codes::INVALID_PARAMS,
                "initialize capabilities must be an object",
            );
        }

        let mut lifecycle = self.lifecycle();
        if *lifecycle != Lifecycle::Uninitialized {
            return Response::error(
                id,
                codes::INVALID_REQUEST,
                "MCP session is already initialized",
            );
        }
        *lifecycle = Lifecycle::AwaitingInitialized;
        drop(lifecycle);

        tracing::debug!(
            client = %params.client_info.name,
            client_version = %params.client_info.version,
            requested_protocol = %params.protocol_version,
            "MCP session initialized"
        );

        Response::result(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "serverInfo": self.info,
                "capabilities": { "tools": {} }
            }),
        )
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

        // A mutating tool is where an external AI could change data. The caller is a
        // language model acting on content it read elsewhere, so a human decides — and
        // a missing confirmer means denial, because no human is available.
        if descriptor.permission.is_mutating() {
            match &self.confirmation {
                Some(service) => {
                    let request = ConfirmationRequest::new(
                        format!("Allow `{name}`?"),
                        format!(
                            "An external AI wants to run `{name}`, which {}data.\n\n{}\n\n\
                             Approve only if you asked for this.",
                            if descriptor.permission == crate::AiPermission::Delete {
                                "deletes "
                            } else {
                                "changes "
                            },
                            descriptor.description,
                        ),
                    );

                    match service.confirm(request).await {
                        Ok(ConfirmationDecision::Approved) => {}
                        Ok(ConfirmationDecision::Denied) => {
                            tracing::warn!(tool = name, "mcp mutating tool denied by the human");
                            return Response::result(
                                id,
                                json!({
                                    "content": [{
                                        "type": "text",
                                        "text": format!(
                                            "The user did not approve running \"{name}\". \
                                             Nothing was changed."
                                        ),
                                    }],
                                    "isError": true,
                                }),
                            );
                        }
                        Err(error) => {
                            // Fail-closed: a broken prompt must not let a tool through.
                            tracing::warn!(tool = name, %error, "confirmation service failed; denying tool call");
                            return Response::result(
                                id,
                                json!({
                                    "content": [{
                                        "type": "text",
                                        "text": format!(
                                            "Confirmation failed for \"{name}\". Nothing \
                                             was changed. ({error})"
                                        ),
                                    }],
                                    "isError": true,
                                }),
                            );
                        }
                    }
                }

                // No confirmer wired: a product that grants mutation but never asks a
                // human. Safe denial as designed.
                None => {
                    tracing::warn!(
                        tool = name,
                        permission = descriptor.permission.as_str(),
                        "mcp mutating tool denied: no confirmation service configured"
                    );
                    return Response::result(
                        id,
                        json!({
                            "content": [{
                                "type": "text",
                                "text": format!(
                                    "Running \"{name}\" requires a human, and the application \
                                     has no confirmation service configured. Grant \
                                     `AiPermission::Commit` only when a confirmer is wired."
                                ),
                            }],
                            "isError": true,
                        }),
                    );
                }
            }
        }

        tracing::info!(
            mcp_session_id = %self.session_id,
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
            Err(error) => {
                let code = if error.is_syntax() || error.is_eof() {
                    codes::PARSE_ERROR
                } else {
                    codes::INVALID_REQUEST
                };
                Some(Response::error(
                    Value::Null,
                    code,
                    format!("malformed request: {error}"),
                ))
            }
        }
    }

    fn lifecycle(&self) -> std::sync::MutexGuard<'_, Lifecycle> {
        self.lifecycle
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AiPermission, AiPermissions, Tool, ToolDescriptor, ToolOutput};
    use async_trait::async_trait;
    use origin_domain::Result;
    use origin_platform::{ConfirmationDecision, ConfirmationRequest, ConfirmationService};
    use std::sync::Arc;

    /// A tool that records whether it was actually called.
    #[derive(Debug)]
    struct TestMutatingTool {
        descriptor: ToolDescriptor,
        called: std::sync::Mutex<bool>,
    }

    impl TestMutatingTool {
        fn new() -> Self {
            Self {
                descriptor: ToolDescriptor::new(
                    "test.mutate",
                    "Test mutation",
                    "A mutating test tool.",
                    AiPermission::Commit,
                ),
                called: std::sync::Mutex::new(false),
            }
        }
    }

    #[async_trait]
    impl Tool for TestMutatingTool {
        fn descriptor(&self) -> ToolDescriptor {
            self.descriptor.clone()
        }

        async fn call(&self, _arguments: serde_json::Value) -> Result<ToolOutput> {
            *self.called.lock().unwrap() = true;
            Ok(ToolOutput::text("done"))
        }
    }

    /// A confirmer that always approves and records requests.
    #[derive(Debug, Default)]
    struct AlwaysApproving {
        requests: std::sync::Mutex<Vec<ConfirmationRequest>>,
    }

    #[async_trait]
    impl ConfirmationService for AlwaysApproving {
        async fn confirm(&self, request: ConfirmationRequest) -> Result<ConfirmationDecision> {
            self.requests.lock().unwrap().push(request.clone());
            Ok(ConfirmationDecision::Approved)
        }
    }

    async fn initialized_server() -> McpServer {
        let server = McpServer::new("test", "0.1.0")
            .with_permissions(AiPermissions::from([
                AiPermission::Read,
                AiPermission::Commit,
            ]))
            .with_tool(Arc::new(TestMutatingTool::new()));

        // Fake an MCP session so tool calls don't fail on lifecycle.
        let request = Request {
            jsonrpc: "2.0".to_owned(),
            id: Some(serde_json::Value::String("1".to_owned())),
            method: "initialize".to_owned(),
            params: serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0.1" }
            }),
        };
        let _ = server.handle(request).await;
        let initialized = Request {
            jsonrpc: "2.0".to_owned(),
            id: None,
            method: "notifications/initialized".to_owned(),
            params: serde_json::Value::Null,
        };
        let _ = server.handle(initialized).await;

        server
    }

    #[tokio::test]
    async fn a_mutating_tool_is_denied_when_no_confirmer_is_wired() {
        let server = initialized_server().await;

        let response = server
            .handle_line(r#"{"jsonrpc":"2.0","id":"t1","method":"tools/call","params":{"name":"test.mutate","arguments":{}}}"#)
            .await
            .expect("must produce a response");

        assert!(response.error.is_none());
        assert!(
            response.result.is_some(),
            "a tool-level error still returns a result with isError:true"
        );
    }

    #[tokio::test]
    async fn a_mutating_tool_is_allowed_when_the_confirmer_approves() {
        let confirmer = Arc::new(AlwaysApproving::default());
        let server = initialized_server()
            .await
            .with_confirmation(confirmer.clone());

        let response = server
            .handle_line(
                r#"{"jsonrpc":"2.0","id":"t2","method":"tools/call","params":{"name":"test.mutate","arguments":{}}}"#,
            )
            .await
            .expect("must produce a response");

        assert!(response.error.is_none());
        assert_eq!(confirmer.requests.lock().unwrap().len(), 1);
    }
}
