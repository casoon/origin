use crate::{Headers, RateLimit};
use origin_core::{AppError, Result};
use serde::de::DeserializeOwned;
use time::OffsetDateTime;

/// One incoming response.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Headers,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn new(status: u16, headers: Headers, body: Vec<u8>) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn rate_limit(&self, now: OffsetDateTime) -> RateLimit {
        RateLimit::from_headers(&self.headers, now)
    }

    pub fn text(&self) -> Result<String> {
        String::from_utf8(self.body.clone())
            .map_err(|error| AppError::ExternalService(format!("response is not utf-8: {error}")))
    }

    pub fn json<T: DeserializeOwned>(&self) -> Result<T> {
        serde_json::from_slice(&self.body).map_err(|error| {
            AppError::ExternalService(format!("unexpected response shape: {error}"))
        })
    }

    /// Turn a non-2xx response into the matching [`AppError`].
    ///
    /// This is the single place where an HTTP status becomes a domain error, so `401`
    /// and `429` mean the same thing to the UI no matter which service produced them.
    /// Callers that treat a specific status as normal — `404` for "not found yet",
    /// `304` for "not modified" — check [`HttpResponse::status`] first.
    pub fn error_for_status(self, now: OffsetDateTime) -> Result<Self> {
        if self.is_success() {
            return Ok(self);
        }

        let rate_limit = self.rate_limit(now);
        let detail = self.error_detail();

        Err(match self.status {
            401 => AppError::Authentication(detail),

            // A 403 with an exhausted budget is a rate limit, not a permission problem.
            // Getting this wrong sends the user to re-authenticate for no reason.
            403 if rate_limit.is_exhausted() || rate_limit.retry_after.is_some() => {
                AppError::RateLimited {
                    message: detail,
                    retry_after_seconds: rate_limit
                        .wait_for(now)
                        .map(|wait| wait.whole_seconds().max(0) as u64),
                }
            }
            403 => AppError::Permission(detail),

            429 => AppError::RateLimited {
                message: detail,
                retry_after_seconds: rate_limit
                    .wait_for(now)
                    .map(|wait| wait.whole_seconds().max(0) as u64),
            },

            400 | 422 => AppError::Validation(detail),
            status => AppError::ExternalService(format!("http {status}: {detail}")),
        })
    }

    /// A short, safe excerpt of the body for the error message.
    ///
    /// Truncated because some services answer with an entire HTML error page, and a
    /// megabyte of markup in a log line helps nobody.
    fn error_detail(&self) -> String {
        const MAX: usize = 200;

        let text = String::from_utf8_lossy(&self.body);
        let trimmed = text.trim();

        if trimmed.is_empty() {
            return format!("http {}", self.status);
        }

        if trimmed.len() <= MAX {
            return trimmed.to_owned();
        }

        let cut = trimmed
            .char_indices()
            .map(|(index, _)| index)
            .take_while(|index| *index <= MAX)
            .last()
            .unwrap_or(0);
        format!("{}…", &trimmed[..cut])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use origin_core::ErrorKind;
    use time::macros::datetime;

    const NOW: OffsetDateTime = datetime!(2026-08-23 10:00 UTC);

    fn response(status: u16, headers: Headers) -> HttpResponse {
        HttpResponse::new(status, headers, b"{}".to_vec())
    }

    #[test]
    fn a_success_passes_through() {
        let response = response(200, Headers::new()).error_for_status(NOW).unwrap();
        assert_eq!(response.status, 200);
    }

    #[test]
    fn unauthorized_becomes_an_authentication_error() {
        let error = response(401, Headers::new())
            .error_for_status(NOW)
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Authentication);
    }

    #[test]
    fn a_forbidden_response_without_budget_left_is_a_rate_limit() {
        let headers =
            Headers::from_iter([("x-ratelimit-remaining", "0"), ("x-ratelimit-reset", "120")]);

        let error = response(403, headers).error_for_status(NOW).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::RateLimited);
        assert_eq!(error.to_contract().retry_after_seconds, Some(120));
    }

    #[test]
    fn a_plain_forbidden_response_stays_a_permission_error() {
        let error = response(403, Headers::new())
            .error_for_status(NOW)
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Permission);
    }

    #[test]
    fn too_many_requests_carries_retry_after() {
        let headers = Headers::from_iter([("retry-after", "30")]);
        let error = response(429, headers).error_for_status(NOW).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::RateLimited);
        assert_eq!(error.to_contract().retry_after_seconds, Some(30));
    }

    #[test]
    fn server_errors_are_external_service_errors() {
        let error = response(503, Headers::new())
            .error_for_status(NOW)
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::ExternalService);
        assert!(error.is_retryable());
    }

    #[test]
    fn a_huge_error_body_is_truncated() {
        let response = HttpResponse::new(500, Headers::new(), "x".repeat(10_000).into_bytes());

        let message = response.error_for_status(NOW).unwrap_err().to_string();

        assert!(message.len() < 300, "message was {} chars", message.len());
        assert!(message.ends_with('…'));
    }
}
