//! The transport, over an in-memory pipe.

use async_trait::async_trait;
use origin_core::Result;
use origin_mcp::{AiPermission, AiPermissions, McpServer, Tool, ToolDescriptor, ToolOutput};
use serde_json::Value;
use std::sync::Arc;
use tokio::io::BufReader;

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

fn server() -> McpServer {
    McpServer::new("demo", "1.0.0")
        .with_permissions(AiPermissions::read_and_propose())
        .with_tool(Arc::new(Echo))
}

/// Feed `input` through the transport and return the response lines.
async fn exchange(input: &str) -> Vec<Value> {
    let mut output = Vec::new();
    origin_mcp_stdio::serve_streams(&server(), BufReader::new(input.as_bytes()), &mut output)
        .await
        .expect("transport");

    String::from_utf8(output)
        .expect("utf-8")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("each line is one JSON object"))
        .collect()
}

#[tokio::test]
async fn a_session_answers_one_line_per_request() {
    let responses = exchange(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"transport-test","version":"1.0.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"demo.echo","arguments":{"a":1}}}
"#,
    )
    .await;

    assert_eq!(responses.len(), 3);
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "demo");
    assert_eq!(responses[1]["result"]["tools"][0]["name"], "demo.echo");
    assert_eq!(responses[2]["result"]["isError"], false);
}

#[tokio::test]
async fn a_notification_gets_no_line_back() {
    let responses = exchange(
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":1,"method":"ping","params":{}}
"#,
    )
    .await;

    assert_eq!(
        responses.len(),
        1,
        "answering a notification is itself a protocol error"
    );
    assert_eq!(responses[0]["id"], 1);
}

#[tokio::test]
async fn blank_lines_are_ignored_rather_than_answered() {
    let responses = exchange("\n\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n\n").await;

    assert_eq!(responses.len(), 1);
}

#[tokio::test]
async fn a_malformed_line_does_not_end_the_session() {
    let responses =
        exchange("not json at all\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n").await;

    assert_eq!(responses.len(), 2);
    assert!(
        responses[0]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("malformed")
    );
    assert_eq!(
        responses[1]["id"], 2,
        "one bad line must not cost the client its session"
    );
}

#[tokio::test]
async fn closing_stdin_ends_the_session_without_an_error() {
    assert!(exchange("").await.is_empty());
}
