//! The loopback HTTP server and a small client for it.
//!
//! One request per connection: read a complete request, answer it, close. The MCP
//! client sends its next request on a fresh connection. That keeps the server tiny and
//! stateless without needing a connection pool.

use crate::activity::Activity;
use crate::discovery::Discovery;
use crate::http::{HttpRequest, HttpResponse, parse_request};
use crate::{MCP_PATH, Token};
use origin_domain::{AppError, Result};
use origin_mcp_core::McpServer;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;

/// A bound loopback endpoint, ready to serve.
#[derive(Debug)]
pub struct HttpTransport {
    listener: TcpListener,
    addr: SocketAddr,
    token: Option<Token>,
    /// When the endpoint last served a request (G18).
    activity: Activity,
}

impl HttpTransport {
    /// Bind `127.0.0.1:0`. The OS picks the port, so two instances never collide.
    pub async fn bind(token: Option<Token>) -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|error| AppError::internal(format!("cannot bind loopback: {error}")))?;
        let addr = listener
            .local_addr()
            .map_err(|error| AppError::internal(format!("cannot read local address: {error}")))?;

        tracing::debug!(%addr, "mcp http endpoint bound");
        Ok(Self {
            listener,
            addr,
            token,
            activity: Activity::default(),
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Activity of this endpoint, for a host that shows an "AI connected" indicator
    /// (G18). Cloneable and safe to read from another task.
    pub fn activity(&self) -> Activity {
        self.activity.clone()
    }

    pub fn url(&self) -> String {
        format!("http://{}{}", self.addr, MCP_PATH)
    }

    pub fn token(&self) -> Option<&Token> {
        self.token.as_ref()
    }

    /// The record a client reads to find this endpoint.
    pub fn discovery(&self) -> Discovery {
        Discovery {
            url: self.url(),
            token: self
                .token
                .as_ref()
                .map(|token| token.expose().to_owned())
                .unwrap_or_default(),
        }
    }

    /// Serve until `stop` is cancelled.
    ///
    /// The server is shared by `Arc` rather than cloned per connection: an MCP
    /// session spans several HTTP requests, so the lifecycle state must survive
    /// between them. (Stdio clones per session because there the connection *is* the
    /// session.)
    pub async fn serve(self, server: Arc<McpServer>, stop: CancellationToken) -> Result<()> {
        tracing::info!(addr = %self.addr, "mcp http endpoint serving");

        loop {
            tokio::select! {
                _ = stop.cancelled() => break,
                accepted = self.listener.accept() => {
                    match accepted {
                        Ok((stream, _peer)) => {
                            let server = server.clone();
                            let token = self.token.clone();
                            let activity = self.activity.clone();
                            tokio::spawn(async move {
                                if let Err(error) = serve_connection(stream, &server, token.as_ref(), &activity).await {
                                    tracing::warn!(%error, "mcp http connection failed");
                                }
                            });
                        }
                        Err(error) => tracing::warn!(%error, "mcp http accept failed"),
                    }
                }
            }
        }

        tracing::debug!("mcp http endpoint stopped");
        Ok(())
    }
}

async fn serve_connection(
    mut stream: TcpStream,
    server: &McpServer,
    token: Option<&Token>,
    activity: &Activity,
) -> Result<()> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];

    loop {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|error| AppError::internal(format!("cannot read request: {error}")))?;

        // Client closed before sending a complete request: nothing to answer.
        if read == 0 {
            return Ok(());
        }

        buffer.extend_from_slice(&chunk[..read]);

        if let Some((request, _consumed)) = parse_request(&buffer)? {
            // A complete request arrived: this is what "a session is active" means.
            activity.touch();
            let response = handle(server, token, &request).await;
            stream
                .write_all(&response.encode())
                .await
                .map_err(|error| AppError::internal(format!("cannot write response: {error}")))?;
            stream
                .flush()
                .await
                .map_err(|error| AppError::internal(format!("cannot flush response: {error}")))?;
            return Ok(());
        }
    }
}

/// Answer one request. Pure apart from the server call, so it is unit-testable.
pub async fn handle(
    server: &McpServer,
    token: Option<&Token>,
    request: &HttpRequest,
) -> HttpResponse {
    if request.path != MCP_PATH {
        return HttpResponse::text(404, format!("unknown path `{}`", request.path));
    }

    if request.method != "POST" {
        return HttpResponse::text(405, "MCP uses POST");
    }

    if let Some(expected) = token {
        match request.bearer_token() {
            Some(candidate) if expected.matches(candidate) => {}
            _ => {
                tracing::warn!("mcp http request rejected: missing or wrong bearer token");
                return HttpResponse::text(401, "unauthorized");
            }
        }
    }

    let body = match std::str::from_utf8(&request.body) {
        Ok(body) => body,
        Err(_) => return HttpResponse::text(400, "request body is not utf-8"),
    };

    match server.handle_line(body).await {
        Some(response) => {
            let value = serde_json::to_value(&response).unwrap_or(serde_json::Value::Null);
            HttpResponse::json(200, &value)
        }
        // A notification: MCP expects no response body.
        None => HttpResponse {
            status: 202,
            body: Vec::new(),
        },
    }
}

/// Send one JSON-RPC message to an endpoint and return the response body.
///
/// Used by the headless start to proxy to a running GUI (G17), and by tests.
pub async fn post(url: &str, token: Option<&str>, body: &str) -> Result<String> {
    let (host, port, path) = split_url(url)?;

    let mut stream = TcpStream::connect((host.as_str(), port))
        .await
        .map_err(|error| AppError::internal(format!("cannot connect to {url}: {error}")))?;

    let mut request = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(value) = token {
        request.push_str("Authorization: Bearer ");
        request.push_str(value);
        request.push_str("\r\n");
    }
    request.push_str("\r\n");

    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|error| AppError::internal(format!("cannot write request: {error}")))?;
    stream
        .write_all(body.as_bytes())
        .await
        .map_err(|error| AppError::internal(format!("cannot write body: {error}")))?;
    stream
        .flush()
        .await
        .map_err(|error| AppError::internal(format!("cannot flush: {error}")))?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .map_err(|error| AppError::internal(format!("cannot read response: {error}")))?;

    let text =
        String::from_utf8(response).map_err(|_| AppError::internal("response is not utf-8"))?;

    let (head, body) = text
        .split_once("\r\n\r\n")
        .ok_or_else(|| AppError::internal("malformed http response"))?;

    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .unwrap_or(0);

    // 200 carries the JSON-RPC response; 202 means the message was a notification,
    // for which MCP expects no body.
    if status != 200 && status != 202 {
        return Err(AppError::internal(format!(
            "endpoint answered with status {status}: {body}"
        )));
    }

    Ok(body.to_owned())
}

fn split_url(url: &str) -> Result<(String, u16, String)> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| AppError::validation(format!("unsupported url scheme: {url}")))?;

    let (authority, path) = match rest.split_once('/') {
        Some((authority, path)) => (authority, format!("/{path}")),
        None => (rest, "/".to_owned()),
    };

    let (host, port) = authority
        .split_once(':')
        .ok_or_else(|| AppError::validation(format!("url has no port: {url}")))?;

    let port = port
        .parse::<u16>()
        .map_err(|_| AppError::validation(format!("invalid port in {url}")))?;

    Ok((host.to_owned(), port, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_url_splits_into_host_port_and_path() {
        let (host, port, path) = split_url("http://127.0.0.1:54321/mcp").unwrap();
        assert_eq!(host, "127.0.0.1");
        assert_eq!(port, 54321);
        assert_eq!(path, "/mcp");
    }

    #[test]
    fn a_url_without_a_port_is_rejected() {
        assert!(split_url("http://127.0.0.1/mcp").is_err());
    }
}
