use crate::Headers;
use crate::headers::RedactedBody;
use origin_domain::{AppError, Result};
use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One outgoing request.
///
/// The body is already encoded bytes: encoding is the caller's decision, and keeping it
/// out of the port means the port never needs to know about JSON, forms or multipart.
#[derive(Clone)]
pub struct HttpRequest {
    pub method: Method,
    pub url: String,
    pub headers: Headers,
    pub body: Option<Vec<u8>>,
}

/// Redacting `Debug`: a request body can carry an OAuth code, a refresh token or a
/// client secret, and a derived `Debug` would print it verbatim into any log line that
/// formats a request with `?`.
impl fmt::Debug for HttpRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpRequest")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("headers", &self.headers)
            .field(
                "body",
                &self.body.as_ref().map(|body| RedactedBody(body.len())),
            )
            .finish()
    }
}

impl HttpRequest {
    pub fn new(method: Method, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
            headers: Headers::new(),
            body: None,
        }
    }

    pub fn get(url: impl Into<String>) -> Self {
        Self::new(Method::Get, url)
    }

    pub fn post(url: impl Into<String>) -> Self {
        Self::new(Method::Post, url)
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Set a bearer token.
    ///
    /// Takes the raw string rather than a `Secret` so that `origin-http` stays
    /// independent of the credential layer; callers pass `token.expose()`.
    pub fn bearer(self, token: &str) -> Self {
        self.header("authorization", format!("Bearer {token}"))
    }

    pub fn json<T: Serialize>(mut self, value: &T) -> Result<Self> {
        let encoded = serde_json::to_vec(value)
            .map_err(|error| AppError::internal(format!("cannot encode request body: {error}")))?;
        self.headers.insert("content-type", "application/json");
        self.body = Some(encoded);
        Ok(self)
    }

    /// `application/x-www-form-urlencoded` body, as used by OAuth token endpoints.
    pub fn form(mut self, fields: &[(&str, &str)]) -> Self {
        let encoded = fields
            .iter()
            .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
            .collect::<Vec<_>>()
            .join("&");

        self.headers
            .insert("content-type", "application/x-www-form-urlencoded");
        self.body = Some(encoded.into_bytes());
        self
    }

    /// Query string appended to the URL.
    pub fn query(mut self, parameters: &[(&str, &str)]) -> Self {
        if parameters.is_empty() {
            return self;
        }

        let encoded = parameters
            .iter()
            .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
            .collect::<Vec<_>>()
            .join("&");

        let separator = if self.url.contains('?') { '&' } else { '?' };
        self.url = format!("{}{separator}{encoded}", self.url);
        self
    }
}

/// Percent-encode for `application/x-www-form-urlencoded`.
///
/// Deliberately conservative: everything outside the unreserved set is escaped, so a
/// scope string, a redirect URI or a PKCE verifier survives intact.
pub(crate) fn encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push_str("%20"),
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_parameters_are_percent_encoded() {
        let request = HttpRequest::get("https://api.example.com/search")
            .query(&[("q", "hello world"), ("scope", "repo:status")]);

        assert_eq!(
            request.url,
            "https://api.example.com/search?q=hello%20world&scope=repo%3Astatus"
        );
    }

    #[test]
    fn debug_output_never_contains_the_body() {
        let request = HttpRequest::post("https://example.com/token")
            .form(&[("refresh_token", "rt-super-secret")]);

        let rendered = format!("{request:?}");

        assert!(!rendered.contains("rt-super-secret"), "got: {rendered}");
        assert!(rendered.contains("bytes, redacted"), "got: {rendered}");
    }

    #[test]
    fn a_request_with_no_body_shows_none() {
        let rendered = format!("{:?}", HttpRequest::get("https://example.com"));
        assert!(rendered.contains("body: None"), "got: {rendered}");
    }

    #[test]
    fn query_appends_to_an_existing_query_string() {
        let request = HttpRequest::get("https://api.example.com/x?page=2").query(&[("per", "50")]);
        assert_eq!(request.url, "https://api.example.com/x?page=2&per=50");
    }

    #[test]
    fn form_bodies_are_encoded_and_typed() {
        let request = HttpRequest::post("https://example.com/token")
            .form(&[("grant_type", "authorization_code"), ("code", "a/b+c")]);

        assert_eq!(
            request.headers.get("content-type"),
            Some("application/x-www-form-urlencoded")
        );
        assert_eq!(
            String::from_utf8(request.body.unwrap()).unwrap(),
            "grant_type=authorization_code&code=a%2Fb%2Bc"
        );
    }
}
