//! The Origin version a project tracks.
//!
//! Only `major.minor.patch` — enough to order migrations, and nothing more, so that a
//! project cannot end up on a version this cannot compare.

use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        let mut parts = value.trim().split('.');

        let mut next = |what: &str| -> Result<u32, String> {
            parts
                .next()
                .ok_or_else(|| format!("version `{value}` has no {what}"))?
                .parse()
                .map_err(|_| format!("version `{value}` has a non-numeric {what}"))
        };

        let version = Self {
            major: next("major")?,
            minor: next("minor")?,
            patch: next("patch")?,
        };

        if parts.next().is_some() {
            return Err(format!("version `{value}` has more than three parts"));
        }

        Ok(version)
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_order_by_component() {
        assert!(Version::parse("0.2.0").unwrap() > Version::parse("0.1.9").unwrap());
        assert!(Version::parse("1.0.0").unwrap() > Version::parse("0.99.99").unwrap());
        assert!(Version::parse("0.1.10").unwrap() > Version::parse("0.1.9").unwrap());
    }

    #[test]
    fn a_version_that_cannot_be_compared_is_rejected_rather_than_guessed() {
        assert!(Version::parse("0.1").is_err());
        assert!(Version::parse("0.1.0-beta").is_err());
        assert!(Version::parse("0.1.0.1").is_err());
    }
}
