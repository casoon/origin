//! Human-confirmation port (G13).
//!
//! A door a mutating action must pass before it takes effect. MCP uses it for
//! `Commit`/`Delete` tools, where the caller is a language model reacting to
//! content from outside and therefore not a trusted actor.
//!
//! The safe default is deny: a headless run without a human, or a product that
//! never wired a real prompt, stays read-only by construction.

use async_trait::async_trait;
use origin_domain::Result;
use std::fmt::Debug;

/// What is being asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmationRequest {
    /// One-line question, e.g. "Run `set.threshold`?"
    pub title: String,
    /// What is being done, and why. Rendered in the dialog body.
    pub body: String,
}

impl ConfirmationRequest {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
        }
    }
}

/// What the human answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationDecision {
    Approved,
    Denied,
}

/// Ask a human before proceeding.
///
/// Implementations return quickly — this is not a long-running approval queue.
/// A read-only product, or one that never wires this capability, gets a deny-all
/// default so that no mutating action can slip through.
#[async_trait]
pub trait ConfirmationService: Debug + Send + Sync + 'static {
    async fn confirm(&self, request: ConfirmationRequest) -> Result<ConfirmationDecision>;
}

/// Fails closed: every request is denied.
///
/// The safe default for headless runs, CLI builds and any product that never wired
/// a real prompt — a product that cannot ask a human cannot let an external AI
/// write through the MCP boundary.
#[derive(Debug, Clone, Copy, Default)]
pub struct DenyingConfirmationService;

#[async_trait]
impl ConfirmationService for DenyingConfirmationService {
    async fn confirm(&self, request: ConfirmationRequest) -> Result<ConfirmationDecision> {
        tracing::info!(
            title = %request.title,
            "confirmation denied — no confirmation service configured"
        );
        Ok(ConfirmationDecision::Denied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_denying_confirmation_service_always_returns_denied() {
        let service = DenyingConfirmationService;

        let decision = service
            .confirm(ConfirmationRequest::new("Allow?", "Run?"))
            .await
            .unwrap();

        assert_eq!(decision, ConfirmationDecision::Denied);
    }
}
