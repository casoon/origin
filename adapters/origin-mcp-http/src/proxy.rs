//! The headless path when a GUI is already running (G17).
//!
//! Starting a second headless instance that opens the same SQLite file as the running
//! GUI invites a class of locking problems. Instead, the headless start looks for a
//! published HTTP endpoint (G16) and, if it is alive, **proxies** its stdio to that
//! endpoint. Stdio then stays the transport a client speaks, but the process doing the
//! work is the one that already owns the database.

use crate::transport::post;
use origin_domain::{AppError, Result};
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};

/// One JSON-RPC ping. Answered without an initialized session, so it is safe to use
/// as a liveness probe.
const PING: &str = r#"{"jsonrpc":"2.0","id":0,"method":"ping","params":{}}"#;

/// Whether an endpoint is reachable and speaking MCP.
pub async fn is_alive(url: &str, token: Option<&str>) -> bool {
    match post(url, token, PING).await {
        Ok(body) => serde_json::from_str::<serde_json::Value>(&body)
            .map(|value| value.get("result").is_some())
            .unwrap_or(false),
        Err(_) => false,
    }
}

/// Read JSON-RPC lines from `input`, forward each to `url`, and write the answers to
/// `output`. Returns when the input ends.
pub async fn proxy_streams<R, W>(
    input: BufReader<R>,
    mut output: W,
    url: &str,
    token: Option<&str>,
) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    tracing::info!(url, "proxying stdio to a running instance over http");

    let mut lines = input.lines();
    loop {
        let line = match lines.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(error) => {
                return Err(AppError::internal(format!("cannot read stdin: {error}")));
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let body = post(url, token, &line).await?;
        if body.trim().is_empty() {
            // A notification: no response is fed back.
            continue;
        }

        output
            .write_all(body.as_bytes())
            .await
            .map_err(|error| AppError::internal(format!("cannot write stdout: {error}")))?;
        output
            .write_all(b"\n")
            .await
            .map_err(|error| AppError::internal(format!("cannot write stdout: {error}")))?;
        output
            .flush()
            .await
            .map_err(|error| AppError::internal(format!("cannot flush stdout: {error}")))?;
    }

    Ok(())
}
