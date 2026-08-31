use crate::random_token;
use base64::Engine as _;
use origin_domain::Result;
use sha2::{Digest, Sha256};

/// Proof Key for Code Exchange (RFC 7636), S256 only.
///
/// `plain` is deliberately not supported: it offers no protection, and every provider
/// worth integrating supports S256.
#[derive(Clone)]
pub struct Pkce {
    verifier: String,
    challenge: String,
}

impl Pkce {
    /// Generate a fresh verifier and its challenge.
    pub fn generate() -> Result<Self> {
        // 32 bytes → 43 base64url characters, the length RFC 7636 recommends.
        let verifier = random_token(32)?;
        let challenge = Self::challenge_for(&verifier);

        Ok(Self {
            verifier,
            challenge,
        })
    }

    fn challenge_for(verifier: &str) -> String {
        let digest = Sha256::digest(verifier.as_bytes());
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
    }

    /// Sent to the authorization endpoint.
    pub fn challenge(&self) -> &str {
        &self.challenge
    }

    /// Sent to the token endpoint. Never appears in a URL the user can see.
    pub fn verifier(&self) -> &str {
        &self.verifier
    }
}

/// Redacted: the verifier is the secret half of the exchange.
impl std::fmt::Debug for Pkce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pkce")
            .field("challenge", &self.challenge)
            .field("verifier", &"***")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_challenge_is_the_sha256_of_the_verifier() {
        // The example pair from RFC 7636, appendix B.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            Pkce::challenge_for(verifier),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn every_flow_gets_a_fresh_verifier() {
        let first = Pkce::generate().unwrap();
        let second = Pkce::generate().unwrap();
        assert_ne!(first.verifier(), second.verifier());
        assert_eq!(first.verifier().len(), 43);
    }

    #[test]
    fn debug_output_never_contains_the_verifier() {
        let pkce = Pkce::generate().unwrap();
        assert!(!format!("{pkce:?}").contains(pkce.verifier()));
    }
}
