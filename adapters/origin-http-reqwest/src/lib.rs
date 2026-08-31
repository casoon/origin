//! `reqwest`-backed implementation of [`origin_http::HttpClient`].
//!
//! One instance per application: `reqwest::Client` owns the connection pool, so
//! constructing several defeats keep-alive and multiplies open sockets.

use async_trait::async_trait;
use origin_domain::{AppError, Result};
use origin_http::{Headers, HttpClient, HttpRequest, HttpResponse};
use std::time::Duration;

/// Default overall request timeout. Long enough for a slow paginated call, short
/// enough that a hung connection cannot stall a sync run indefinitely.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Default connect timeout. A separate, much shorter budget, so an unreachable host
/// fails fast instead of consuming the whole request timeout.
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Default ceiling on a response body. Comfortably large for any JSON API response
/// Origin talks to; small enough that a runaway or hostile server cannot exhaust the
/// desktop process's memory reading an unbounded or falsely-labelled body.
const DEFAULT_MAX_RESPONSE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ReqwestHttpClient {
    inner: reqwest::Client,
    max_response_bytes: u64,
}

impl ReqwestHttpClient {
    /// Build a client identified by `user_agent`.
    ///
    /// Several APIs reject or throttle requests without a meaningful user agent, so it
    /// is required rather than optional.
    pub fn new(user_agent: impl AsRef<str>) -> Result<Self> {
        Self::builder(user_agent).build()
    }

    pub fn builder(user_agent: impl AsRef<str>) -> ReqwestHttpClientBuilder {
        ReqwestHttpClientBuilder {
            user_agent: user_agent.as_ref().to_owned(),
            timeout: DEFAULT_TIMEOUT,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReqwestHttpClientBuilder {
    user_agent: String,
    timeout: Duration,
    connect_timeout: Duration,
    max_response_bytes: u64,
}

impl ReqwestHttpClientBuilder {
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn connect_timeout(mut self, connect_timeout: Duration) -> Self {
        self.connect_timeout = connect_timeout;
        self
    }

    /// Override the response body ceiling (default 10 MiB). A connector whose provider
    /// legitimately answers with larger payloads sets this explicitly and visibly,
    /// rather than the port having no limit at all.
    pub fn max_response_bytes(mut self, max_response_bytes: u64) -> Self {
        self.max_response_bytes = max_response_bytes;
        self
    }

    pub fn build(self) -> Result<ReqwestHttpClient> {
        let inner = reqwest::Client::builder()
            .user_agent(self.user_agent)
            .timeout(self.timeout)
            .connect_timeout(self.connect_timeout)
            .build()
            .map_err(|error| {
                AppError::configuration(format!("cannot build http client: {error}"))
            })?;

        Ok(ReqwestHttpClient {
            inner,
            max_response_bytes: self.max_response_bytes,
        })
    }
}

#[async_trait]
impl HttpClient for ReqwestHttpClient {
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse> {
        let method = reqwest::Method::from_bytes(request.method.as_str().as_bytes())
            .map_err(|error| AppError::internal(format!("invalid http method: {error}")))?;

        // The URL is logged with its query string stripped — a query can carry an API
        // key or an OAuth code — and headers are not logged at all; `Headers` only
        // redacts when something formats it with `Debug`.
        tracing::debug!(
            method = %request.method,
            url = %request.url.split('?').next().unwrap_or(&request.url),
            "http request"
        );

        let mut builder = self.inner.request(method, &request.url);
        for (name, value) in request.headers.iter() {
            builder = builder.header(name, value);
        }
        if let Some(body) = request.body {
            builder = builder.body(body);
        }

        let response = builder.send().await.map_err(to_app_error)?;

        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                // A header we cannot read as text is dropped rather than fatal: it is
                // never one we route on.
                value
                    .to_str()
                    .ok()
                    .map(|value| (name.as_str().to_owned(), value.to_owned()))
            })
            .collect::<Headers>();

        let body = read_body_limited(response, self.max_response_bytes).await?;

        tracing::debug!(status, bytes = body.len(), "http response");
        Ok(HttpResponse::new(status, headers, body))
    }
}

/// Reads a response body up to `limit` bytes, failing rather than buffering further.
///
/// A `Content-Length` header is checked first so an honestly-labelled oversized
/// response fails before any of it is read; the running total during the read guards
/// against a response with no `Content-Length` or one that undercounts it.
async fn read_body_limited(mut response: reqwest::Response, limit: u64) -> Result<Vec<u8>> {
    if let Some(length) = response.content_length()
        && length > limit
    {
        return Err(AppError::ExternalService(format!(
            "response declared {length} bytes, over the {limit} byte limit"
        )));
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(to_app_error)? {
        if body.len() as u64 + chunk.len() as u64 > limit {
            return Err(AppError::ExternalService(format!(
                "response body exceeds the {limit} byte limit"
            )));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// Transport failures only — a non-2xx status is not an error here (see [`HttpClient`]).
///
/// The offline/network distinction matters: `Offline` tells the sync engine to wait for
/// connectivity, while `Network` means retry with backoff.
fn to_app_error(error: reqwest::Error) -> AppError {
    if error.is_timeout() {
        return AppError::Network(format!("request timed out: {error}"));
    }

    if error.is_connect() {
        return AppError::Offline(format!("cannot reach host: {error}"));
    }

    AppError::Network(error.to_string())
}
