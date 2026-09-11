//! MCP over a local loopback HTTP endpoint (G16).
//!
//! The stdio transport covers "the app is not running yet": the client starts the
//! application as a child process. It does **not** cover the more common case where the
//! GUI is already open and an MCP-capable client wants to attach to the running
//! instance. This adapter fills that gap.
//!
//! ```text
//!   CLI / stdio (no GUI running)          HTTP loopback (GUI already running)
//!   ────────────────────────────          ─────────────────────────────────
//!   client ──spawn──▶ app ──stdio         client ──POST──▶ 127.0.0.1:<port>/mcp
//! ```
//!
//! The endpoint binds to `127.0.0.1:0` — the same pattern as the OAuth redirect
//! (ADR-0015): the OS picks a free port, so two instances never collide. The port is
//! published in a discovery file so a client can find it, and access is gated by a
//! bearer token (G19).

mod activity;
mod discovery;
mod http;
mod proxy;
mod transport;

pub use activity::Activity;
pub use discovery::Discovery;
pub use http::{HttpRequest, HttpResponse, parse_request};
pub use proxy::{is_alive, proxy_streams};
pub use transport::{HttpTransport, handle, post};

/// The path an MCP client posts to. One well-known path keeps the discovery file
/// trivial.
pub const MCP_PATH: &str = "/mcp";

/// A bearer token guarding the endpoint (G19).
///
/// A locally negotiated secret: the GUI shows a confirmation, then hands the token to
/// the client. Compared in constant time so a local attacker cannot probe it byte by
/// byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token(String);

impl Token {
    /// Generate a fresh random token.
    pub fn generate() -> Self {
        // 256 bits from the OS CSPRNG. Preferring `getrandom` over a UUID keeps the
        // entropy source explicit and the token a fixed width.
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).expect("the OS random source must be available");
        let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        Self(hex)
    }

    pub fn from_string(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Constant-time equality: no early return on the first differing byte.
    pub fn matches(&self, candidate: &str) -> bool {
        let expected = self.0.as_bytes();
        let candidate = candidate.as_bytes();
        if expected.len() != candidate.len() {
            return false;
        }
        let mut diff = 0u8;
        for (a, b) in expected.iter().zip(candidate.iter()) {
            diff |= a ^ b;
        }
        diff == 0
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token(***)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_generated_token_is_long_and_unpredictable() {
        let first = Token::generate();
        let second = Token::generate();

        assert_eq!(first.expose().len(), 64);
        assert_ne!(first, second);
    }

    #[test]
    fn token_equality_is_exact() {
        let token = Token::from_string("secret-value");

        assert!(token.matches("secret-value"));
        assert!(!token.matches("secret-valuX"));
        assert!(!token.matches("secret"));
        assert!(!token.matches(""));
    }

    #[test]
    fn the_display_does_not_leak_the_token() {
        let token = Token::from_string("super-secret");
        assert_eq!(token.to_string(), "Token(***)");
    }
}
