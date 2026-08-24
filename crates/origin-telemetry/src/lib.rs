//! Logging and observability.
//!
//! Logging is a platform concern, not something every product configures from scratch.
//!
//! # Rules
//!
//! - Never log a secret. `Secret` redacts itself in `Debug`; do not undo that by
//!   logging `secret.expose()`.
//! - Never log personal data unfiltered — email addresses, repository contents,
//!   analytics dimensions that identify a person.
//! - Attach correlation fields ([`spans`]) instead of writing ids into the message.

pub mod spans;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;

/// Log output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    /// Human-readable. The default for development.
    #[default]
    Pretty,
    /// One JSON object per line, for shipped builds and log files.
    Json,
}

#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Fallback filter when `RUST_LOG` is unset, e.g. `"info,origin_sync=debug"`.
    pub default_filter: String,
    pub format: Format,
    /// Log span open/close, which makes sync and job timings visible.
    pub log_span_events: bool,

    /// Write to stderr instead of stdout.
    ///
    /// Mandatory when the process speaks a protocol on stdout — an MCP server over
    /// stdio, for instance. A single log line on stdout corrupts the stream, and the
    /// client reports a parse error rather than anything that points at logging.
    pub to_stderr: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            default_filter: "info".to_owned(),
            format: Format::default(),
            log_span_events: false,
            to_stderr: false,
        }
    }
}

impl TelemetryConfig {
    /// Configuration for a process that speaks a protocol on stdout.
    pub fn for_stdout_protocol() -> Self {
        Self {
            to_stderr: true,
            ..Self::default()
        }
    }
}

/// Install the global tracing subscriber.
///
/// Returns `false` if a subscriber was already installed — which happens in test
/// binaries and is harmless, so this never panics.
pub fn init(config: TelemetryConfig) -> bool {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.default_filter));

    let span_events = if config.log_span_events {
        FmtSpan::NEW | FmtSpan::CLOSE
    } else {
        FmtSpan::NONE
    };

    macro_rules! install {
        ($builder:expr) => {
            match config.format {
                Format::Pretty => $builder.try_init().is_ok(),
                Format::Json => $builder.json().try_init().is_ok(),
            }
        };
    }

    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_span_events(span_events)
        .with_target(true);

    if config.to_stderr {
        install!(builder.with_writer(std::io::stderr))
    } else {
        install!(builder)
    }
}
