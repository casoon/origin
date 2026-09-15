//! The loopback HTTP server and a small client for it.
//!
//! One request per connection: read a complete request, answer it, close. The MCP
//! client sends its next request on a fresh connection. That keeps the server tiny
//! without needing a connection pool.
//!
//! Sessions follow the MCP Streamable HTTP transport: `initialize` opens a session, the
//! response names it in the `Mcp-Session-Id` header, and every later request carries
//! that header. Each session has its own lifecycle, so a second client — or the same
//! client reconnecting — never collides with an earlier `initialize`.

use crate::activity::Activity;
use crate::discovery::Discovery;
use crate::http::{HttpRequest, HttpResponse, parse_request};
use crate::{MCP_PATH, Token};
use origin_domain::{AppError, Result};
use origin_mcp_core::McpServer;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;

/// The header that names an MCP session.
pub const SESSION_HEADER: &str = "Mcp-Session-Id";

/// Sessions kept at once. A client that disappears without a word must not grow the
/// table forever: when it is full, the least recently used session goes, and its
/// client gets a 404 — which the specification defines as "initialize again".
const MAX_SESSIONS: usize = 32;

/// How long a client may take to deliver one complete request.
///
/// Only reading is bounded. Answering may legitimately take much longer — a mutating
/// tool waits for the user to confirm.
const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// A bound loopback endpoint, ready to serve.
#[derive(Debug)]
pub struct HttpTransport {
    listener: TcpListener,
    addr: SocketAddr,
    token: Token,
    /// When the endpoint last served a request (G18).
    activity: Activity,
}

impl HttpTransport {
    /// Bind `127.0.0.1:0`. The OS picks the port, so two instances never collide.
    ///
    /// The token is not optional: every local process — and every web page, through
    /// the browser — can reach a loopback port.
    pub async fn bind(token: Token) -> Result<Self> {
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

    pub fn token(&self) -> &Token {
        &self.token
    }

    /// The record a client reads to find this endpoint.
    pub fn discovery(&self) -> Discovery {
        Discovery {
            url: self.url(),
            token: self.token.expose().to_owned(),
        }
    }

    /// Serve until `stop` is cancelled.
    ///
    /// `server` is a template that is never initialised itself: every session gets its
    /// own clone, and with it its own lifecycle.
    pub async fn serve(self, server: Arc<McpServer>, stop: CancellationToken) -> Result<()> {
        tracing::info!(addr = %self.addr, "mcp http endpoint serving");
        let endpoint = Arc::new(Endpoint::new(server, self.token, self.addr.port()));

        loop {
            tokio::select! {
                _ = stop.cancelled() => break,
                accepted = self.listener.accept() => {
                    match accepted {
                        Ok((stream, _peer)) => {
                            let endpoint = endpoint.clone();
                            let activity = self.activity.clone();
                            tokio::spawn(async move {
                                if let Err(error) = serve_connection(stream, &endpoint, &activity).await {
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
    endpoint: &Endpoint,
    activity: &Activity,
) -> Result<()> {
    let request = tokio::time::timeout(READ_TIMEOUT, read_request(&mut stream))
        .await
        .map_err(|_| AppError::internal("client did not send a complete request in time"))??;

    // Client closed before sending a complete request: nothing to answer.
    let Some(request) = request else {
        return Ok(());
    };

    // A complete request arrived: this is what "a session is active" means.
    activity.touch();
    let response = endpoint.handle(&request).await;
    stream
        .write_all(&response.encode())
        .await
        .map_err(|error| AppError::internal(format!("cannot write response: {error}")))?;
    stream
        .flush()
        .await
        .map_err(|error| AppError::internal(format!("cannot flush response: {error}")))?;
    Ok(())
}

async fn read_request(stream: &mut TcpStream) -> Result<Option<HttpRequest>> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];

    loop {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|error| AppError::internal(format!("cannot read request: {error}")))?;

        if read == 0 {
            return Ok(None);
        }

        buffer.extend_from_slice(&chunk[..read]);

        if let Some((request, _consumed)) = parse_request(&buffer)? {
            return Ok(Some(request));
        }
    }
}

/// Answers the requests of one bound endpoint: host and origin checks, the token, and
/// the session table. Separate from the socket, so it is unit-testable.
#[derive(Debug)]
pub struct Endpoint {
    /// Never initialised itself; see [`HttpTransport::serve`].
    template: Arc<McpServer>,
    token: Token,
    port: u16,
    sessions: Mutex<Sessions>,
}

#[derive(Debug, Default)]
struct Sessions {
    entries: HashMap<String, Session>,
    /// Monotonic use counter, for evicting the least recently used session.
    clock: u64,
}

#[derive(Debug)]
struct Session {
    server: Arc<McpServer>,
    last_used: u64,
}

impl Endpoint {
    pub fn new(server: Arc<McpServer>, token: Token, port: u16) -> Self {
        Self {
            template: server,
            token,
            port,
            sessions: Mutex::new(Sessions::default()),
        }
    }

    /// Answer one request.
    pub async fn handle(&self, request: &HttpRequest) -> HttpResponse {
        if request.path != MCP_PATH {
            return HttpResponse::text(404, format!("unknown path `{}`", request.path));
        }

        if request.method != "POST" {
            return HttpResponse::text(405, "MCP uses POST");
        }

        if !self.is_loopback_request(request) {
            tracing::warn!("mcp http request rejected: foreign host or origin");
            return HttpResponse::text(403, "forbidden");
        }

        match request.bearer_token() {
            Some(candidate) if self.token.matches(candidate) => {}
            _ => {
                tracing::warn!("mcp http request rejected: missing or wrong bearer token");
                return HttpResponse::text(401, "unauthorized");
            }
        }

        let body = match std::str::from_utf8(&request.body) {
            Ok(body) => body,
            Err(_) => return HttpResponse::text(400, "request body is not utf-8"),
        };

        if let Some(id) = request.header(SESSION_HEADER) {
            return match self.session(id) {
                Some(server) => to_http(server.handle_line(body).await),
                None => HttpResponse::text(404, "unknown or expired MCP session; initialize again"),
            };
        }

        match method_of(body).as_deref() {
            Some("initialize") => self.open_session(body).await,
            // A liveness probe needs no session: `ping` is answered in any state.
            Some("ping") => to_http(self.template.handle_line(body).await),
            _ => HttpResponse::text(
                400,
                format!("missing `{SESSION_HEADER}` header; send `initialize` first"),
            ),
        }
    }

    /// Whether `Host` and, if present, `Origin` name this endpoint.
    ///
    /// A browser attaches `Origin` to every POST, so a web page is refused before its
    /// token guess is even looked at. A DNS-rebinding page reaches the port under its
    /// own host name, which the `Host` check catches. Native MCP clients send no
    /// `Origin`.
    fn is_loopback_request(&self, request: &HttpRequest) -> bool {
        let is_this_endpoint = |authority: &str| {
            ["127.0.0.1", "localhost"]
                .iter()
                .any(|host| authority.eq_ignore_ascii_case(&format!("{host}:{}", self.port)))
        };

        let host = request.header("host").is_some_and(is_this_endpoint);
        let origin = request
            .header("origin")
            .is_none_or(|origin| origin.strip_prefix("http://").is_some_and(is_this_endpoint));
        host && origin
    }

    async fn open_session(&self, body: &str) -> HttpResponse {
        let server = Arc::new((*self.template).clone());
        let Some(response) = server.handle_line(body).await else {
            return HttpResponse::accepted();
        };

        let value = serde_json::to_value(&response).unwrap_or(Value::Null);
        // A refused initialize opens nothing.
        if value.get("error").is_some_and(|error| !error.is_null()) {
            return HttpResponse::json(200, &value);
        }

        let id = new_session_id();
        tracing::debug!(mcp_session_id = %server.session_id(), "mcp http session opened");
        self.insert(id.clone(), server);
        HttpResponse::json(200, &value).with_header(SESSION_HEADER, id)
    }

    fn session(&self, id: &str) -> Option<Arc<McpServer>> {
        let mut sessions = self.sessions();
        sessions.clock += 1;
        let now = sessions.clock;
        let session = sessions.entries.get_mut(id)?;
        session.last_used = now;
        Some(session.server.clone())
    }

    fn insert(&self, id: String, server: Arc<McpServer>) {
        let mut sessions = self.sessions();
        if sessions.entries.len() >= MAX_SESSIONS {
            let oldest = sessions
                .entries
                .iter()
                .min_by_key(|(_, session)| session.last_used)
                .map(|(id, _)| id.clone());
            if let Some(oldest) = oldest {
                sessions.entries.remove(&oldest);
                tracing::debug!("mcp http session evicted: too many sessions");
            }
        }

        sessions.clock += 1;
        let last_used = sessions.clock;
        sessions.entries.insert(id, Session { server, last_used });
    }

    fn sessions(&self) -> MutexGuard<'_, Sessions> {
        self.sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
}

fn to_http(response: Option<impl Serialize>) -> HttpResponse {
    match response {
        Some(response) => {
            let value = serde_json::to_value(&response).unwrap_or(Value::Null);
            HttpResponse::json(200, &value)
        }
        // A notification: MCP expects no response body.
        None => HttpResponse::accepted(),
    }
}

fn method_of(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    value.get("method")?.as_str().map(str::to_owned)
}

fn new_session_id() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("the OS random source must be available");
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A client for one MCP session.
///
/// Remembers the session the endpoint assigns on `initialize` and sends it with every
/// later request. Used by the headless start to proxy to a running GUI (G17), and by
/// tests.
#[derive(Debug, Clone)]
pub struct Client {
    url: String,
    token: Option<String>,
    session: Option<String>,
}

impl Client {
    pub fn new(url: &str, token: Option<&str>) -> Self {
        Self {
            url: url.to_owned(),
            token: token.map(str::to_owned),
            session: None,
        }
    }

    /// The session the endpoint assigned, once `initialize` went through.
    pub fn session(&self) -> Option<&str> {
        self.session.as_deref()
    }

    /// Send one JSON-RPC message and return the response body — empty for a
    /// notification.
    pub async fn send(&mut self, body: &str) -> Result<String> {
        let reply = exchange(
            &self.url,
            self.token.as_deref(),
            self.session.as_deref(),
            body,
        )
        .await?;
        if let Some(session) = reply.session {
            self.session = Some(session);
        }
        Ok(reply.body)
    }
}

/// Send one message outside any session and return the response body.
///
/// Enough for a liveness `ping`; anything after `initialize` needs a [`Client`].
pub async fn post(url: &str, token: Option<&str>, body: &str) -> Result<String> {
    exchange(url, token, None, body)
        .await
        .map(|reply| reply.body)
}

struct Reply {
    body: String,
    session: Option<String>,
}

async fn exchange(
    url: &str,
    token: Option<&str>,
    session: Option<&str>,
    body: &str,
) -> Result<Reply> {
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
    if let Some(value) = session {
        request.push_str(SESSION_HEADER);
        request.push_str(": ");
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

    let mut head_lines = head.lines();
    let status = head_lines
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

    let session = head_lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case(SESSION_HEADER))
        .map(|(_, value)| value.trim().to_owned());

    Ok(Reply {
        body: body.to_owned(),
        session,
    })
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
