//! HTTP as a port (ADR-0014).
//!
//! Connectors and SDK crates depend on [`HttpClient`], never on a concrete HTTP
//! library. That keeps one connection pool and one timeout policy for the whole
//! application, gives rate limits and status mapping a single home, and makes
//! connector tests deterministic and offline.

mod client;
mod headers;
mod rate_limit;
mod request;
mod response;

#[cfg(feature = "testing")]
pub mod testing;

pub use client::HttpClient;
pub use headers::Headers;
pub use rate_limit::RateLimit;
pub use request::{HttpRequest, Method};
pub use response::HttpResponse;
