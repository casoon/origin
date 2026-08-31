use crate::{HttpRequest, HttpResponse};
use async_trait::async_trait;
use origin_domain::Result;
use std::fmt::Debug;

/// The HTTP port.
///
/// Implementations own connection pooling, timeouts and redirects. They must translate
/// transport failures into [`origin_domain::AppError`]:
///
/// - no route, DNS failure, connection refused → `Offline` or `Network`
/// - timeout → `Network`
/// - anything else → `Network`
///
/// They must **not** turn a non-2xx status into an error; that is the caller's decision
/// via [`HttpResponse::error_for_status`], because some statuses are normal.
#[async_trait]
pub trait HttpClient: Debug + Send + Sync + 'static {
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse>;
}
