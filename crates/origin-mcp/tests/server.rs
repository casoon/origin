//! The boundary: what an external AI can see and invoke, and what it cannot.

use async_trait::async_trait;
use origin_domain::{AppError, Result};
use origin_mcp::{AiPermission, AiPermissions, McpServer, Tool, ToolDescriptor, ToolOutput};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug)]
struct Recorded {
    name: &'static str,
    permission: AiPermission,
    calls: AtomicU32,
    fails: bool,
}

impl Recorded {
    fn new(name: &'static str, permission: AiPermission) -> Arc<Self> {
        Arc::new(Self {
            name,
            permission,
            calls: AtomicU32::new(0),
            fails: false,
        })
    }

    fn failing(name: &'static str, permission: AiPermission) -> Arc<Self> {
        Arc::new(Self {
            name,
            permission,
            calls: AtomicU32::new(0),
            fails: true,
        })
    }

    fn calls(&self) -> u32 {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Tool for Recorded {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor::new(
            self.name,
            self.name,
            "A tool used in tests.",
            self.permission,
        )
    }

    async fn call(&self, arguments: Value) -> Result<ToolOutput> {
        self.calls.fetch_add(1, Ordering::SeqCst);

        if self.fails {
            return Err(AppError::ExternalService("the service is down".to_owned()));
        }

        Ok(ToolOutput::text(format!("called with {arguments}")))
    }
}

fn request(id: u32, method: &str, params: Value) -> origin_mcp::Request {
    serde_json::from_value(json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    }))
    .expect("valid request")
}

fn result(response: &origin_mcp::Response) -> &Value {
    response.result.as_ref().expect("a result, not an error")
}

fn initialize_params() -> Value {
    json!({
        "protocolVersion": origin_mcp::PROTOCOL_VERSION,
        "capabilities": {},
        "clientInfo": { "name": "origin-tests", "version": "1.0.0" }
    })
}

async fn ready(server: McpServer) -> McpServer {
    let response = server
        .handle(request(0, "initialize", initialize_params()))
        .await
        .expect("initialize response");
    assert!(response.error.is_none(), "initialization failed");

    let notification = serde_json::from_value(json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    }))
    .unwrap();
    assert!(server.handle(notification).await.is_none());
    server
}

#[tokio::test]
async fn a_server_announces_itself_and_its_tools() {
    let server = McpServer::new("demo", "1.0.0")
        .with_permissions(AiPermissions::read_and_propose())
        .with_tool(Recorded::new("demo.read", AiPermission::Read));

    let response = server
        .handle(request(1, "initialize", json!({})))
        .await
        .unwrap();

    assert_eq!(response.error.as_ref().unwrap().code, -32602);

    let response = server
        .handle(request(2, "initialize", initialize_params()))
        .await
        .unwrap();
    assert_eq!(result(&response)["serverInfo"]["name"], "demo");
    assert!(result(&response)["capabilities"]["tools"].is_object());
}

#[tokio::test]
async fn tools_beyond_the_grant_are_never_even_listed() {
    let server = ready(
        McpServer::new("demo", "1.0.0")
            .with_permissions(AiPermissions::read_and_propose())
            .with_tool(Recorded::new("demo.read", AiPermission::Read))
            .with_tool(Recorded::new("demo.propose", AiPermission::Propose))
            .with_tool(Recorded::new("demo.delete", AiPermission::Delete)),
    )
    .await;

    let response = server
        .handle(request(1, "tools/list", json!({})))
        .await
        .unwrap();
    let names: Vec<&str> = result(&response)["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();

    assert_eq!(names, vec!["demo.propose", "demo.read"]);
    assert!(
        !names.contains(&"demo.delete"),
        "advertising a tool that always fails teaches the model to retry"
    );
}

#[tokio::test]
async fn a_permitted_call_reaches_the_tool() {
    let tool = Recorded::new("demo.read", AiPermission::Read);
    let server = ready(
        McpServer::new("demo", "1.0.0")
            .with_permissions(AiPermissions::read_and_propose())
            .with_tool(tool.clone()),
    )
    .await;

    let response = server
        .handle(request(
            1,
            "tools/call",
            json!({ "name": "demo.read", "arguments": { "id": 7 } }),
        ))
        .await
        .unwrap();

    assert_eq!(tool.calls(), 1);
    assert_eq!(result(&response)["isError"], false);
    assert!(
        result(&response)["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("\"id\":7")
    );
}

#[tokio::test]
async fn a_call_beyond_the_grant_never_reaches_the_tool() {
    let tool = Recorded::new("demo.delete", AiPermission::Delete);
    let server = ready(
        McpServer::new("demo", "1.0.0")
            .with_permissions(AiPermissions::read_and_propose())
            .with_tool(tool.clone()),
    )
    .await;

    let response = server
        .handle(request(1, "tools/call", json!({ "name": "demo.delete" })))
        .await
        .unwrap();

    assert_eq!(
        tool.calls(),
        0,
        "prompt injection in a document must not be able to delete anything"
    );
    let error = response.error.expect("a protocol error");
    assert!(error.message.contains("delete"), "got: {}", error.message);
}

#[tokio::test]
async fn a_failing_tool_is_reported_to_the_model_rather_than_to_the_transport() {
    let server = ready(
        McpServer::new("demo", "1.0.0")
            .with_permissions(AiPermissions::read_and_propose())
            .with_tool(Recorded::failing("demo.read", AiPermission::Read)),
    )
    .await;

    let response = server
        .handle(request(1, "tools/call", json!({ "name": "demo.read" })))
        .await
        .unwrap();

    assert!(
        response.error.is_none(),
        "a tool failure is not a protocol failure"
    );
    assert_eq!(result(&response)["isError"], true);
    assert!(
        result(&response)["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("the service is down"),
        "the model has to be able to read what went wrong and choose differently"
    );
}

#[tokio::test]
async fn an_unknown_tool_is_refused_by_name() {
    let server =
        ready(McpServer::new("demo", "1.0.0").with_permissions(AiPermissions::read_and_propose()))
            .await;

    let response = server
        .handle(request(1, "tools/call", json!({ "name": "demo.nope" })))
        .await
        .unwrap();

    assert!(response.error.unwrap().message.contains("demo.nope"));
}

#[tokio::test]
async fn a_server_with_no_grant_exposes_nothing() {
    let server = ready(
        McpServer::new("demo", "1.0.0").with_tool(Recorded::new("demo.read", AiPermission::Read)),
    )
    .await;

    let response = server
        .handle(request(1, "tools/list", json!({})))
        .await
        .unwrap();

    assert!(
        result(&response)["tools"].as_array().unwrap().is_empty(),
        "MCP is off until a product switches it on"
    );
}

#[tokio::test]
async fn a_notification_produces_no_response() {
    let server = McpServer::new("demo", "1.0.0");

    server
        .handle(request(1, "initialize", initialize_params()))
        .await
        .unwrap();

    let notification: origin_mcp::Request =
        serde_json::from_value(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
            .unwrap();

    assert!(server.handle(notification).await.is_none());
}

#[tokio::test]
async fn an_unknown_method_is_refused_without_dropping_the_connection() {
    let server = ready(McpServer::new("demo", "1.0.0")).await;

    let response = server
        .handle(request(1, "resources/list", json!({})))
        .await
        .unwrap();

    assert_eq!(response.error.unwrap().code, -32601);
}

#[tokio::test]
async fn a_malformed_line_produces_an_error_rather_than_a_panic() {
    let server = McpServer::new("demo", "1.0.0");

    let response = server.handle_line("{ this is not json").await.unwrap();

    let error = response.error.unwrap();
    assert_eq!(error.code, -32700);
    assert!(error.message.contains("malformed"));
}

#[tokio::test]
async fn tools_are_refused_until_the_initialized_notification_arrives() {
    let server = McpServer::new("demo", "1.0.0");

    let before_initialize = server
        .handle(request(1, "tools/list", json!({})))
        .await
        .unwrap();
    assert_eq!(before_initialize.error.unwrap().code, -32600);

    server
        .handle(request(2, "initialize", initialize_params()))
        .await
        .unwrap();
    let before_notification = server
        .handle(request(3, "tools/list", json!({})))
        .await
        .unwrap();
    assert_eq!(before_notification.error.unwrap().code, -32600);
}

#[tokio::test]
async fn a_non_json_rpc_request_is_invalid() {
    let server = McpServer::new("demo", "1.0.0");
    let request = serde_json::from_value(json!({
        "jsonrpc": "1.0",
        "id": 1,
        "method": "ping"
    }))
    .unwrap();

    let response = server.handle(request).await.unwrap();
    assert_eq!(response.error.unwrap().code, -32600);
}

#[tokio::test]
async fn a_request_without_a_json_rpc_version_is_invalid() {
    let server = McpServer::new("demo", "1.0.0");
    let response = server
        .handle_line(r#"{"id":1,"method":"ping"}"#)
        .await
        .unwrap();

    assert_eq!(response.error.unwrap().code, -32600);
}
