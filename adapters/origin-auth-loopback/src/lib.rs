//! Loopback redirect listener (ADR-0015, RFC 8252).
//!
//! Binds an ephemeral port on `127.0.0.1`, serves exactly the one redirect the
//! provider sends, and shuts down. No custom URL scheme registration, no
//! platform-specific behaviour, and testable with a plain TCP client.

mod query;

use async_trait::async_trait;
use origin_auth::{AuthorizationCode, RedirectListener};
use origin_domain::{AppError, Result};
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Path the provider redirects to.
const CALLBACK_PATH: &str = "/callback";

/// How long to wait for the user to finish in the browser.
///
/// Without a limit, a user who closes the tab leaves the listener — and whatever is
/// awaiting it — alive for the rest of the session.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

/// Largest request we will read. A redirect is a few hundred bytes; anything larger is
/// not the browser we are waiting for.
const MAX_REQUEST_BYTES: usize = 8 * 1024;

#[derive(Debug)]
pub struct LoopbackRedirect {
    listener: TcpListener,
    redirect_uri: String,
    timeout: Duration,
}

impl LoopbackRedirect {
    /// Bind an ephemeral port.
    ///
    /// Ephemeral by default so two Origin applications authorizing at the same time
    /// cannot collide.
    pub async fn bind() -> Result<Self> {
        Self::bind_port(0).await
    }

    /// Bind a fixed port, for providers that require the redirect URI to be registered
    /// in advance and reject a dynamic port.
    pub async fn bind_port(port: u16) -> Result<Self> {
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))
            .await
            .map_err(|error| {
                AppError::configuration(format!("cannot bind loopback redirect port: {error}"))
            })?;

        let address = listener.local_addr().map_err(|error| {
            AppError::internal(format!("cannot read loopback redirect address: {error}"))
        })?;

        let redirect_uri = format!("http://127.0.0.1:{}{CALLBACK_PATH}", address.port());
        tracing::debug!(%redirect_uri, "loopback redirect listening");

        Ok(Self {
            listener,
            redirect_uri,
            timeout: DEFAULT_TIMEOUT,
        })
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    async fn accept_redirect(&self, expected_state: &str) -> Result<AuthorizationCode> {
        loop {
            let (stream, _) = self.listener.accept().await.map_err(|error| {
                AppError::internal(format!("loopback redirect accept failed: {error}"))
            })?;

            match self.handle(stream, expected_state).await {
                // Browsers request `/favicon.ico` and sometimes pre-connect. Those are
                // not the redirect, so keep waiting instead of failing the flow.
                Ok(None) => continue,
                Ok(Some(code)) => return Ok(code),
                Err(error) => return Err(error),
            }
        }
    }

    /// Returns `Ok(None)` for a request that is not the redirect we are waiting for.
    async fn handle(
        &self,
        mut stream: TcpStream,
        expected_state: &str,
    ) -> Result<Option<AuthorizationCode>> {
        let request_line = read_request_line(&mut stream).await?;
        let Some(target) = request_line.split_whitespace().nth(1) else {
            respond(&mut stream, 400, "Bad request").await;
            return Ok(None);
        };

        let (path, query) = target.split_once('?').unwrap_or((target, ""));
        if path != CALLBACK_PATH {
            respond(&mut stream, 404, "Not found").await;
            return Ok(None);
        }

        let parameters = query::parse(query);
        let get = |name: &str| {
            parameters
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, v)| v.clone())
        };

        // The state check is what makes this listener safe: anything on localhost can
        // hit this port, but only the flow we started knows the state.
        match get("state") {
            Some(state) if state == expected_state => {}
            _ => {
                respond(
                    &mut stream,
                    400,
                    "Unexpected request. You can close this window.",
                )
                .await;
                return Err(AppError::Authentication(
                    "redirect did not carry the expected state — the flow was not started \
                     by this application"
                        .to_owned(),
                ));
            }
        }

        if let Some(error) = get("error") {
            let description = get("error_description").unwrap_or_else(|| error.clone());
            respond(
                &mut stream,
                400,
                "Authorization was denied. You can close this window.",
            )
            .await;
            return Err(AppError::Authentication(format!(
                "authorization was denied: {description}"
            )));
        }

        let Some(code) = get("code") else {
            respond(&mut stream, 400, "Missing authorization code.").await;
            return Err(AppError::Authentication(
                "redirect carried no authorization code".to_owned(),
            ));
        };

        respond(
            &mut stream,
            200,
            "Signed in. You can close this window and return to the app.",
        )
        .await;
        Ok(Some(AuthorizationCode::new(code)))
    }
}

#[async_trait]
impl RedirectListener for LoopbackRedirect {
    fn redirect_uri(&self) -> String {
        self.redirect_uri.clone()
    }

    async fn wait(&self, expected_state: &str) -> Result<AuthorizationCode> {
        tokio::time::timeout(self.timeout, self.accept_redirect(expected_state))
            .await
            .map_err(|_| {
                AppError::Authentication(
                    "timed out waiting for the browser to complete authorization".to_owned(),
                )
            })?
    }
}

/// Read up to the end of the request line.
///
/// Only the first line is needed; the headers and body of a redirect carry nothing we
/// use, and reading them would mean parsing HTTP properly.
async fn read_request_line(stream: &mut TcpStream) -> Result<String> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 1024];

    loop {
        let read = stream.read(&mut chunk).await.map_err(|error| {
            AppError::internal(format!("cannot read loopback redirect request: {error}"))
        })?;

        if read == 0 {
            break;
        }

        buffer.extend_from_slice(&chunk[..read]);

        if let Some(end) = buffer.iter().position(|byte| *byte == b'\n') {
            buffer.truncate(end);
            break;
        }

        if buffer.len() > MAX_REQUEST_BYTES {
            return Err(AppError::internal(
                "loopback redirect request exceeded the size limit".to_owned(),
            ));
        }
    }

    Ok(String::from_utf8_lossy(&buffer).trim_end().to_owned())
}

/// Best-effort response. The user's browser showing a blank page is a cosmetic problem;
/// the authorization itself already succeeded or failed by this point.
async fn respond(stream: &mut TcpStream, status: u16, message: &str) {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        _ => "Not Found",
    };

    let body = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <title>Origin</title><style>body{{font:16px system-ui;display:grid;\
         place-items:center;height:100vh;margin:0;color:#14171c;background:#f6f7f9}}\
         @media(prefers-color-scheme:dark){{body{{color:#e7eaef;background:#0f1115}}}}\
         </style></head><body><p>{message}</p></body></html>"
    );

    let response = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         content-type: text/html; charset=utf-8\r\n\
         content-length: {}\r\n\
         connection: close\r\n\r\n{body}",
        body.len()
    );

    if let Err(error) = stream.write_all(response.as_bytes()).await {
        tracing::debug!(%error, "cannot write loopback redirect response");
    }
    let _ = stream.flush().await;
    let _ = stream.shutdown().await;
}
