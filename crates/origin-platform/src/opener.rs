use async_trait::async_trait;
use origin_core::Result;
use std::fmt::Debug;

/// Opens a URL in the user's browser.
///
/// Scoped deliberately narrowly: this is *not* a general shell escape. Anything that
/// executes local programs needs its own contract and its own capability (ADR-0007).
#[async_trait]
pub trait Opener: Debug + Send + Sync + 'static {
    async fn open_url(&self, url: &str) -> Result<()>;
}
