//! Just enough HTTP/1.1 to carry JSON-RPC over loopback.
//!
//! Deliberately minimal: one request per connection, `Content-Length` only (no
//! chunked encoding), and hard caps on header and body size so a local process cannot
//! exhaust memory. Anything more belongs in a general-purpose HTTP stack, which this
//! adapter deliberately does not pull in.

use origin_domain::{AppError, Result};

/// Largest request line + headers we will buffer.
const MAX_HEADER_BYTES: usize = 64 * 1024;

/// Largest body we will read. MCP tool arguments are small; a megabyte is generous.
const MAX_BODY_BYTES: usize = 1024 * 1024;

const HEADER_TERMINATOR: &[u8] = b"\r\n\r\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    /// Header names lowercased for case-insensitive lookup.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.as_str())
    }

    /// The bearer token from `Authorization: Bearer <token>`, if present.
    pub fn bearer_token(&self) -> Option<&str> {
        self.header("authorization")?
            .strip_prefix("Bearer ")
            .map(str::trim)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn json(status: u16, value: &serde_json::Value) -> Self {
        let body = serde_json::to_vec(value).unwrap_or_else(|_| b"null".to_vec());
        Self { status, body }
    }

    pub fn text(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            body: message.into().into_bytes(),
        }
    }

    fn reason(&self) -> &'static str {
        match self.status {
            200 => "OK",
            400 => "Bad Request",
            401 => "Unauthorized",
            404 => "Not Found",
            405 => "Method Not Allowed",
            413 => "Payload Too Large",
            500 => "Internal Server Error",
            _ => "Error",
        }
    }

    /// Encode as an HTTP/1.1 response, always closing the connection.
    pub fn encode(&self) -> Vec<u8> {
        let head = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.status,
            self.reason(),
            self.body.len()
        );
        let mut out = head.into_bytes();
        out.extend_from_slice(&self.body);
        out
    }
}

/// Parse one request from `buffer`.
///
/// Returns `Ok(None)` when more bytes are needed; `Ok(Some((request, consumed)))` once
/// the full request (headers + `Content-Length` body) is present.
pub fn parse_request(buffer: &[u8]) -> Result<Option<(HttpRequest, usize)>> {
    let Some(header_end) = find_terminator(buffer) else {
        if buffer.len() > MAX_HEADER_BYTES {
            return Err(AppError::validation("http headers exceed the size limit"));
        }
        return Ok(None);
    };

    let head = std::str::from_utf8(&buffer[..header_end])
        .map_err(|_| AppError::validation("http headers are not utf-8"))?;

    let mut lines = head.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| AppError::validation("empty http request"))?;

    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| AppError::validation("missing http method"))?
        .to_owned();
    let path = parts
        .next()
        .ok_or_else(|| AppError::validation("missing http path"))?
        .to_owned();

    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once(':')
            .ok_or_else(|| AppError::validation("malformed http header"))?;
        headers.push((key.trim().to_ascii_lowercase(), value.trim().to_owned()));
    }

    let content_length = headers
        .iter()
        .find(|(key, _)| key == "content-length")
        .map(|(_, value)| {
            value
                .parse::<usize>()
                .map_err(|_| AppError::validation("invalid content-length"))
        })
        .transpose()?
        .unwrap_or(0);

    if content_length > MAX_BODY_BYTES {
        return Err(AppError::validation("http body exceeds the size limit"));
    }

    let body_start = header_end + HEADER_TERMINATOR.len();
    let body_end = body_start + content_length;
    if buffer.len() < body_end {
        return Ok(None);
    }

    Ok(Some((
        HttpRequest {
            method,
            path,
            headers,
            body: buffer[body_start..body_end].to_vec(),
        },
        body_end,
    )))
}

fn find_terminator(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(HEADER_TERMINATOR.len())
        .position(|window| window == HEADER_TERMINATOR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_complete_post_is_parsed() {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"POST /mcp HTTP/1.1");
        raw.extend_from_slice(b"\r\n");
        raw.extend_from_slice(b"Host: 127.0.0.1");
        raw.extend_from_slice(b"\r\n");
        raw.extend_from_slice(b"Authorization: Bearer abc");
        raw.extend_from_slice(b"\r\n");
        raw.extend_from_slice(b"Content-Length: 7");
        raw.extend_from_slice(b"\r\n\r\n");
        raw.extend_from_slice(b"{\"a\":1}");

        let (request, consumed) = parse_request(&raw).unwrap().expect("complete request");

        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/mcp");
        assert_eq!(request.body, b"{\"a\":1}");
        assert_eq!(request.bearer_token(), Some("abc"));
        assert_eq!(consumed, raw.len());
    }

    #[test]
    fn an_incomplete_request_asks_for_more() {
        let raw = b"POST /mcp HTTP/1.1\r\nContent-Length: 20\r\n\r\n{\"a\"";
        assert!(parse_request(raw).unwrap().is_none());
    }

    #[test]
    fn a_request_without_a_body_parses() {
        let raw = b"GET /mcp HTTP/1.1\r\nHost: x\r\n\r\n";
        let (request, _) = parse_request(raw).unwrap().expect("complete request");
        assert_eq!(request.method, "GET");
        assert!(request.body.is_empty());
    }

    #[test]
    fn a_missing_bearer_prefix_yields_no_token() {
        let raw = b"POST /mcp HTTP/1.1\r\nAuthorization: Basic xyz\r\n\r\n";
        let (request, _) = parse_request(raw).unwrap().unwrap();
        assert_eq!(request.bearer_token(), None);
    }

    #[test]
    fn a_response_encodes_a_content_length_and_closes() {
        let response = HttpResponse::text(401, "nope");
        let encoded = String::from_utf8(response.encode()).unwrap();

        assert!(encoded.starts_with("HTTP/1.1 401 Unauthorized\r\n"));
        assert!(encoded.contains("Content-Length: 4\r\n"));
        assert!(encoded.ends_with("\r\n\r\nnope"));
    }

    #[test]
    fn an_oversized_body_is_rejected() {
        let raw = format!(
            "POST /mcp HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
            MAX_BODY_BYTES + 1
        );
        assert!(parse_request(raw.as_bytes()).is_err());
    }
}
