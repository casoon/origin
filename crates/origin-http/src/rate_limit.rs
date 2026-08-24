use crate::Headers;
use time::{Duration, OffsetDateTime};

/// What a service told us about our remaining budget.
///
/// Parsed once, here, from the three conventions that cover almost every API in
/// practice: the IETF `RateLimit-*` draft headers, the widespread `X-RateLimit-*`
/// variants, and plain `Retry-After`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RateLimit {
    /// Requests allowed in the current window.
    pub limit: Option<u64>,
    /// Requests still available.
    pub remaining: Option<u64>,
    /// When the window resets.
    pub reset_at: Option<OffsetDateTime>,
    /// How long the service asked us to wait before retrying.
    pub retry_after: Option<Duration>,
}

impl RateLimit {
    /// Read rate-limit metadata from response headers.
    ///
    /// `now` is needed because `Retry-After` and some `reset` headers are relative;
    /// it comes from the `Clock` port so this stays testable.
    pub fn from_headers(headers: &Headers, now: OffsetDateTime) -> Self {
        let limit = headers
            .get_u64("ratelimit-limit")
            .or_else(|| headers.get_u64("x-ratelimit-limit"));

        let remaining = headers
            .get_u64("ratelimit-remaining")
            .or_else(|| headers.get_u64("x-ratelimit-remaining"));

        let retry_after = headers
            .get_u64("retry-after")
            .map(|seconds| Duration::seconds(seconds as i64));

        let reset_at =
            Self::reset_at(headers, now).or_else(|| retry_after.map(|after| now + after));

        Self {
            limit,
            remaining,
            reset_at,
            retry_after,
        }
    }

    /// `reset` is a delta in the IETF draft and an absolute unix timestamp in the
    /// GitHub-style headers. Values that look like a timestamp are treated as one.
    fn reset_at(headers: &Headers, now: OffsetDateTime) -> Option<OffsetDateTime> {
        /// Anything above this is a unix timestamp, not "seconds from now".
        /// (2001-09-09; no sane API asks a client to wait 31 years.)
        const TIMESTAMP_THRESHOLD: u64 = 1_000_000_000;

        let value = headers
            .get_u64("x-ratelimit-reset")
            .or_else(|| headers.get_u64("ratelimit-reset"))?;

        if value >= TIMESTAMP_THRESHOLD {
            OffsetDateTime::from_unix_timestamp(value as i64).ok()
        } else {
            Some(now + Duration::seconds(value as i64))
        }
    }

    /// Whether the budget is exhausted.
    pub fn is_exhausted(&self) -> bool {
        self.remaining == Some(0)
    }

    /// How long to wait before the next attempt, if the service said anything useful.
    pub fn wait_for(&self, now: OffsetDateTime) -> Option<Duration> {
        if let Some(retry_after) = self.retry_after {
            return Some(retry_after);
        }

        if !self.is_exhausted() {
            return None;
        }

        self.reset_at
            .map(|reset_at| (reset_at - now).max(Duration::ZERO))
    }

    /// Whether this response carried any rate-limit information at all.
    pub fn is_present(&self) -> bool {
        self.limit.is_some() || self.remaining.is_some() || self.retry_after.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    const NOW: OffsetDateTime = datetime!(2026-08-23 10:00 UTC);

    #[test]
    fn github_style_headers_are_understood() {
        // GitHub reports `reset` as an absolute unix timestamp.
        let reset = (NOW + Duration::minutes(30)).unix_timestamp().to_string();
        let headers = Headers::from_iter([
            ("x-ratelimit-limit", "5000".to_owned()),
            ("x-ratelimit-remaining", "0".to_owned()),
            ("x-ratelimit-reset", reset),
        ]);

        let limit = RateLimit::from_headers(&headers, NOW);

        assert_eq!(limit.limit, Some(5000));
        assert!(limit.is_exhausted());
        assert_eq!(limit.wait_for(NOW), Some(Duration::minutes(30)));
    }

    #[test]
    fn ietf_style_reset_is_relative() {
        let headers = Headers::from_iter([("ratelimit-remaining", "0"), ("ratelimit-reset", "60")]);

        let limit = RateLimit::from_headers(&headers, NOW);

        assert_eq!(limit.reset_at, Some(NOW + Duration::seconds(60)));
        assert_eq!(limit.wait_for(NOW), Some(Duration::seconds(60)));
    }

    #[test]
    fn retry_after_wins_over_a_reset_window() {
        let headers = Headers::from_iter([
            ("ratelimit-remaining", "0"),
            ("ratelimit-reset", "600"),
            ("retry-after", "20"),
        ]);

        assert_eq!(
            RateLimit::from_headers(&headers, NOW).wait_for(NOW),
            Some(Duration::seconds(20))
        );
    }

    #[test]
    fn a_healthy_budget_asks_for_no_wait() {
        let headers = Headers::from_iter([("x-ratelimit-remaining", "4999")]);
        assert_eq!(RateLimit::from_headers(&headers, NOW).wait_for(NOW), None);
    }

    #[test]
    fn a_response_without_rate_limit_headers_is_reported_as_absent() {
        assert!(!RateLimit::from_headers(&Headers::new(), NOW).is_present());
    }
}
