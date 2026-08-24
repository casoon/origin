//! Test doubles for the AI port.

use crate::{AiService, Completion, Prompt, Usage};
use async_trait::async_trait;
use origin_core::{AppError, Result};
use std::collections::VecDeque;
use std::sync::Mutex;

/// Answers from a queue and records what it was asked.
///
/// Lets AI features be tested without a network, without a key and without cost — and
/// makes the prompts themselves reviewable, which matters because a prompt is the part
/// most likely to be wrong.
#[derive(Debug, Default)]
pub struct RecordingAiService {
    answers: Mutex<VecDeque<Result<String>>>,
    prompts: Mutex<Vec<Prompt>>,
}

impl RecordingAiService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&self, answer: impl Into<String>) -> &Self {
        self.answers
            .lock()
            .expect("recorder poisoned")
            .push_back(Ok(answer.into()));
        self
    }

    pub fn push_error(&self, error: AppError) -> &Self {
        self.answers
            .lock()
            .expect("recorder poisoned")
            .push_back(Err(error));
        self
    }

    /// Every prompt that was sent, in order.
    pub fn prompts(&self) -> Vec<Prompt> {
        self.prompts.lock().expect("recorder poisoned").clone()
    }
}

#[async_trait]
impl AiService for RecordingAiService {
    fn model(&self) -> &str {
        "recording"
    }

    async fn complete(&self, prompt: Prompt) -> Result<Completion> {
        self.prompts
            .lock()
            .expect("recorder poisoned")
            .push(prompt.clone());

        let text = self
            .answers
            .lock()
            .expect("recorder poisoned")
            .pop_front()
            .unwrap_or_else(|| {
                Err(AppError::internal(format!(
                    "no answer queued for `{}`",
                    prompt.instruction
                )))
            })?;

        Ok(Completion {
            usage: Usage {
                input_tokens: prompt.input.len() as u32 / 4,
                output_tokens: text.len() as u32 / 4,
            },
            truncated: false,
            model: "recording".to_owned(),
            text,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn prompts_are_recorded_so_they_can_be_reviewed() {
        let service = RecordingAiService::new();
        service.push("A short summary.");

        let completion = service
            .complete(Prompt::new("Summarise this note", "a long note"))
            .await
            .unwrap();

        assert_eq!(completion.text, "A short summary.");
        assert_eq!(service.prompts()[0].instruction, "Summarise this note");
        assert_eq!(service.prompts()[0].temperature, 0.0);
    }

    #[tokio::test]
    async fn an_unqueued_call_fails_loudly_rather_than_returning_nothing() {
        let service = RecordingAiService::new();

        let error = service.complete(Prompt::new("x", "y")).await.unwrap_err();

        assert!(error.to_string().contains("no answer queued"));
    }
}
