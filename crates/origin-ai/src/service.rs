use crate::{Completion, Prompt};
use async_trait::async_trait;
use origin_core::{AppError, Result};
use std::fmt::Debug;

/// Inference, as a port.
///
/// Implementations live in `adapters/origin-ai-*`. Domain code never names a provider,
/// so switching one — or letting the user pick — changes the composition root and
/// nothing else.
///
/// Errors follow the usual model: a provider outage is `ExternalService`, a rejected
/// key is `Authentication`, a quota is `RateLimited`. Callers can therefore treat an
/// unavailable model exactly like an unavailable API.
#[async_trait]
pub trait AiService: Debug + Send + Sync + 'static {
    /// The model this service will use, for display and for the record.
    fn model(&self) -> &str;

    async fn complete(&self, prompt: Prompt) -> Result<Completion>;
}

/// Refuses every request.
///
/// The default for a product with AI features switched off, and for anything that
/// must not silently reach the network. It fails rather than returning an empty
/// answer: a feature that quietly produces nothing is harder to diagnose than one that
/// says it is unavailable.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopAiService;

#[async_trait]
impl AiService for NoopAiService {
    fn model(&self) -> &str {
        "none"
    }

    async fn complete(&self, _prompt: Prompt) -> Result<Completion> {
        Err(AppError::configuration(
            "this application has no AI provider configured",
        ))
    }
}
