//! The HTTP transport, over a real loopback socket.

use async_trait::async_trait;
use origin_domain::Result;
use origin_mcp_core::{AiPermission, AiPermissions, McpServer, Tool, ToolDescriptor, ToolOutput};
use origin_mcp_http::{HttpTransport, Token, post};
use serde_json::Value;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
struct Echo;

#[async_trait]
impl Tool for Echo {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor::new(
            "demo.echo",
            "Echo",
            "Repeats its input.",
            AiPermission::Read,
        )
    }

    async fn call(&self, arguments: Value) -> Result<ToolOutput> {
        Ok(ToolOutput::text(arguments.to_string()))
    }
}

fn server() -> Arc<McpServer> {
    Arc::new(
        McpServer::new("demo", "1.0.0")
            .with_permissions(AiPermissions::read_and_propose())
            .with_tool(Arc::new(Echo)),
    )
}

async fn spawn_transport(token: Option<Token>) -> (String, CancellationToken) {
    let transport = HttpTransport::bind(token).await.expect("bind");
    let url = transport.url();
    let stop = CancellationToken::new();

    let task_stop = stop.clone();
    tokio::spawn(async move {
        let _ = transport.serve(server(), task_stop).await;
    });

    (url, stop)
}

const INITIALIZE: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"http-test","version":"1.0.0"}}}"#;
const INITIALIZED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
const LIST: &str = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;

#[tokio::test]
async fn a_session_round_trips_over_http() {
    let (url, stop) = spawn_transport(None).await;

    // A session spans several requests, so the server must keep its lifecycle state.
    let init = post(&url, None, INITIALIZE).await.unwrap();
    let init: Value = serde_json::from_str(&init).unwrap();
    assert_eq!(init["result"]["serverInfo"]["name"], "demo");

    let _ = post(&url, None, INITIALIZED).await.unwrap();

    let list = post(&url, None, LIST).await.unwrap();
    let list: Value = serde_json::from_str(&list).unwrap();
    assert_eq!(list["result"]["tools"][0]["name"], "demo.echo");

    stop.cancel();
}

#[tokio::test]
async fn a_missing_token_is_rejected_when_one_is_required() {
    let token = Token::from_string("expected-token");
    let (url, stop) = spawn_transport(Some(token.clone())).await;

    let error = post(&url, None, INITIALIZE).await.unwrap_err();
    assert!(
        error.to_string().contains("401"),
        "a request without the token must be refused, got: {error}"
    );

    // The right token works.
    post(&url, Some(token.expose()), INITIALIZE).await.unwrap();

    stop.cancel();
}

#[tokio::test]
async fn a_wrong_token_is_rejected() {
    let (url, stop) = spawn_transport(Some(Token::from_string("right"))).await;

    let error = post(&url, Some("wrong"), INITIALIZE).await.unwrap_err();
    assert!(error.to_string().contains("401"), "got: {error}");

    stop.cancel();
}
