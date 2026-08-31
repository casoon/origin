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

#[derive(Debug, Clone)]
pub struct ReqwestHttpClient {
    inner: reqwest::Client,
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
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReqwestHttpClientBuilder {
    user_agent: String,
    timeout: Duration,
    connect_timeout: Duration,
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

    pub fn build(self) -> Result<ReqwestHttpClient> {
        let inner = reqwest::Client::builder()
            .user_agent(self.user_agent)
            .timeout(self.timeout)
            .connect_timeout(self.connect_timeout)
            .build()
            .map_err(|error| {
                AppError::configuration(format!("cannot build http client: {error}"))
            })?;

        Ok(ReqwestHttpClient { inner })
    }
}

#[async_trait]
impl HttpClient for ReqwestHttpClient {
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse> {
        let method = reqwest::Method::from_bytes(request.method.as_str().as_bytes())
            .map_err(|error| AppError::internal(format!("invalid http method: {error}")))?;

        // The URL is logged; headers are not — they carry the bearer token, and
        // `Headers` only redacts when something formats it with `Debug`.
        tracing::debug!(method = %request.method, url = %request.url, "http request");

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

        let body = response.bytes().await.map_err(to_app_error)?.to_vec();

        tracing::debug!(status, bytes = body.len(), "http response");
        Ok(HttpResponse::new(status, headers, body))
    }
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
