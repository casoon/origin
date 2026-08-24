use std::fmt;

/// Header names whose values must never reach a log line.
const SENSITIVE: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "x-auth-token",
];

/// Case-insensitive header collection.
///
/// Stored as a list rather than a map because HTTP allows repeated headers and the
/// order occasionally matters.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Headers(Vec<(String, String)>);

impl Headers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into().to_ascii_lowercase();
        self.0.retain(|(existing, _)| existing != &name);
        self.0.push((name, value.into()));
    }

    pub fn append(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.0
            .push((name.into().to_ascii_lowercase(), value.into()));
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.0
            .iter()
            .find(|(existing, _)| existing == &name)
            .map(|(_, value)| value.as_str())
    }

    /// Parse a header value as a number, ignoring anything unparseable.
    ///
    /// A malformed rate-limit header is not worth failing a request over.
    pub fn get_u64(&self, name: &str) -> Option<u64> {
        self.get(name)?.trim().parse().ok()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl<K: Into<String>, V: Into<String>> FromIterator<(K, V)> for Headers {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(entries: I) -> Self {
        let mut headers = Self::new();
        for (name, value) in entries {
            headers.append(name, value);
        }
        headers
    }
}

/// Redacting `Debug`.
///
/// A `tracing::debug!(?headers, ...)` anywhere in the stack would otherwise print
/// bearer tokens into the log file.
impl fmt::Debug for Headers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map = f.debug_map();
        for (name, value) in self.iter() {
            if SENSITIVE.contains(&name) {
                map.entry(&name, &"***");
            } else {
                map.entry(&name, &value);
            }
        }
        map.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_ignores_case() {
        let mut headers = Headers::new();
        headers.insert("Content-Type", "application/json");

        assert_eq!(headers.get("content-type"), Some("application/json"));
        assert_eq!(headers.get("CONTENT-TYPE"), Some("application/json"));
    }

    #[test]
    fn insert_replaces_while_append_keeps_both() {
        let mut headers = Headers::new();
        headers.insert("x-test", "a");
        headers.insert("x-test", "b");
        assert_eq!(headers.len(), 1);

        headers.append("x-test", "c");
        assert_eq!(headers.len(), 2);
    }

    #[test]
    fn debug_output_redacts_credentials() {
        let mut headers = Headers::new();
        headers.insert("Authorization", "Bearer ghp_supersecret");
        headers.insert("Accept", "application/json");

        let rendered = format!("{headers:?}");

        assert!(!rendered.contains("ghp_supersecret"), "got: {rendered}");
        assert!(rendered.contains("application/json"));
    }

    #[test]
    fn a_malformed_numeric_header_is_ignored_rather_than_fatal() {
        let headers = Headers::from_iter([("x-ratelimit-remaining", "unknown")]);
        assert_eq!(headers.get_u64("x-ratelimit-remaining"), None);
    }
}
