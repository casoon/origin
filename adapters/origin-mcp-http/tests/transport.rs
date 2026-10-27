//! The HTTP transport, over a real loopback socket.

use async_trait::async_trait;
use origin_domain::Result;
use origin_mcp_core::{AiPermission, AiPermissions, McpServer, Tool, ToolDescriptor, ToolOutput};
use origin_mcp_http::{Client, HttpTransport, Token, is_alive, post};
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
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

const TOKEN: &str = "test-token";

async fn spawn_transport() -> (String, CancellationToken) {
    let transport = HttpTransport::bind(Token::from_string(TOKEN))
        .await
        .expect("bind");
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

/// Open a session the way a client does.
async fn open_session(url: &str) -> Client {
    let mut client = Client::new(url, Some(TOKEN));

    let init: Value = serde_json::from_str(&client.send(INITIALIZE).await.unwrap()).unwrap();
    assert_eq!(init["result"]["serverInfo"]["name"], "demo");
    assert!(client.session().is_some(), "initialize must open a session");

    client.send(INITIALIZED).await.unwrap();
    client
}

async fn first_tool(client: &mut Client) -> Value {
    let list: Value = serde_json::from_str(&client.send(LIST).await.unwrap()).unwrap();
    list["result"]["tools"][0]["name"].clone()
}

/// The authority part of the endpoint URL, e.g. `127.0.0.1:54321`.
fn authority(url: &str) -> String {
    url.trim_start_matches("http://")
        .trim_end_matches("/mcp")
        .to_owned()
}

/// Send an `initialize` with hand-picked `Host` and `Origin` headers — what a browser
/// would send, which the client never does. Returns the status line.
async fn raw(url: &str, host: &str, origin: Option<&str>) -> String {
    let mut stream = TcpStream::connect(authority(url)).await.expect("connect");

    let mut request = format!("POST /mcp HTTP/1.1\r\nHost: {host}\r\n");
    if let Some(origin) = origin {
        request.push_str(&format!("Origin: {origin}\r\n"));
    }
    request.push_str(&format!(
        "Authorization: Bearer {TOKEN}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{INITIALIZE}",
        INITIALIZE.len()
    ));
    stream.write_all(request.as_bytes()).await.expect("write");

    let mut response = String::new();
    stream.read_to_string(&mut response).await.expect("read");
    response.lines().next().unwrap_or_default().to_owned()
}

#[tokio::test]
async fn a_session_round_trips_over_http() {
    let (url, stop) = spawn_transport().await;

    let mut client = open_session(&url).await;
    assert_eq!(first_tool(&mut client).await, "demo.echo");

    stop.cancel();
}

#[tokio::test]
async fn two_clients_hold_independent_sessions() {
    let (url, stop) = spawn_transport().await;

    // Before sessions, the second `initialize` was refused as "already initialized" —
    // so no second client, and no reconnect, could ever attach.
    let mut first = open_session(&url).await;
    let mut second = open_session(&url).await;
    assert_ne!(first.session(), second.session());

    assert_eq!(first_tool(&mut first).await, "demo.echo");
    assert_eq!(first_tool(&mut second).await, "demo.echo");

    stop.cancel();
}

#[tokio::test]
async fn a_request_outside_a_session_is_refused() {
    let (url, stop) = spawn_transport().await;

    let error = post(&url, Some(TOKEN), LIST).await.unwrap_err();
    assert!(error.to_string().contains("400"), "got: {error}");

    stop.cancel();
}

#[tokio::test]
async fn a_liveness_ping_needs_no_session() {
    let (url, stop) = spawn_transport().await;

    assert!(is_alive(&url, Some(TOKEN)).await);

    stop.cancel();
}

#[tokio::test]
async fn a_missing_token_is_rejected() {
    let (url, stop) = spawn_transport().await;

    let error = post(&url, None, INITIALIZE).await.unwrap_err();
    assert!(
        error.to_string().contains("401"),
        "a request without the token must be refused, got: {error}"
    );

    // The right token works.
    post(&url, Some(TOKEN), INITIALIZE).await.unwrap();

    stop.cancel();
}

#[tokio::test]
async fn a_wrong_token_is_rejected() {
    let (url, stop) = spawn_transport().await;

    let error = post(&url, Some("wrong"), INITIALIZE).await.unwrap_err();
    assert!(error.to_string().contains("401"), "got: {error}");

    stop.cancel();
}

#[tokio::test]
async fn a_web_page_origin_is_refused() {
    let (url, stop) = spawn_transport().await;

    let status = raw(&url, &authority(&url), Some("https://evil.example")).await;
    assert!(status.starts_with("HTTP/1.1 403"), "got: {status}");

    stop.cancel();
}

#[tokio::test]
async fn a_foreign_host_is_refused() {
    let (url, stop) = spawn_transport().await;

    // What a DNS-rebinding page sends: our port, its own host name.
    let port = authority(&url).rsplit(':').next().unwrap().to_owned();
    let status = raw(&url, &format!("evil.example:{port}"), None).await;
    assert!(status.starts_with("HTTP/1.1 403"), "got: {status}");

    stop.cancel();
}

#[tokio::test]
async fn the_endpoint_s_own_origin_is_accepted() {
    let (url, stop) = spawn_transport().await;

    let own = format!("http://{}", authority(&url));
    let status = raw(&url, &authority(&url), Some(&own)).await;
    assert!(status.starts_with("HTTP/1.1 200"), "got: {status}");

    stop.cancel();
}
