//! MCP over stdio.
//!
//! The client starts the application as a child process and talks to it over its
//! standard streams — no port to allocate, no token to exchange, and it works while
//! the GUI is not running.
//!
//! # stdout belongs to the protocol
//!
//! Nothing else may write there. A single log line corrupts the stream and the client
//! reports a parse error, which points nowhere near the actual cause. Configure
//! logging with [`TelemetryConfig::for_stdout_protocol`] before serving.
//!
//! [`TelemetryConfig::for_stdout_protocol`]: https://docs.rs/origin-telemetry

use origin_domain::{AppError, Result};
use origin_mcp::McpServer;
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};

/// Serve until the client closes stdin.
pub async fn serve(server: &McpServer) -> Result<()> {
    let input = BufReader::new(tokio::io::stdin());
    let output = tokio::io::stdout();

    serve_streams(server, input, output).await
}

/// Serve over arbitrary streams. Used by the tests, which is the point of the split.
pub async fn serve_streams<R, W>(
    server: &McpServer,
    input: BufReader<R>,
    mut output: W,
) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    tracing::debug!("mcp stdio transport started");
    let mut lines = input.lines();

    loop {
        let line = match lines.next_line().await {
            Ok(Some(line)) => line,
            // Client closed the stream: an ordinary end, not a failure.
            Ok(None) => break,
            Err(error) => {
                return Err(AppError::internal(format!(
                    "cannot read from stdin: {error}"
                )));
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let Some(response) = server.handle_line(&line).await else {
            // A notification. Answering one would itself be a protocol error.
            continue;
        };

        let encoded = serde_json::to_string(&response)
            .map_err(|error| AppError::internal(format!("cannot encode response: {error}")))?;

        output
            .write_all(encoded.as_bytes())
            .await
            .map_err(write_error)?;
        output.write_all(b"\n").await.map_err(write_error)?;

        // Flushed per message: the client is waiting for this answer before it sends
        // the next request, so a buffered response deadlocks both sides.
        output.flush().await.map_err(write_error)?;
    }

    tracing::debug!("mcp stdio transport stopped");
    Ok(())
}

fn write_error(error: std::io::Error) -> AppError {
    AppError::internal(format!("cannot write to stdout: {error}"))
}
