use serde::{Deserialize, Serialize};

/// What the application asks a model to do.
///
/// Deliberately not a chat transcript: most product features are one instruction over
/// one piece of content, and modelling them as conversations invites state nobody
/// needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    /// What the model should do.
    pub instruction: String,
    /// What it should do it to.
    pub input: String,
    /// Upper bound on the answer. A product that does not cap this is one bad prompt
    /// away from a surprising bill.
    pub max_output_tokens: u32,
    /// `0.0` for extraction and classification, higher for drafting.
    pub temperature: f32,
}

impl Prompt {
    /// A deterministic prompt: temperature zero, short answer.
    ///
    /// The right default for classification and extraction, which is most of what a
    /// desktop application actually needs.
    pub fn new(instruction: impl Into<String>, input: impl Into<String>) -> Self {
        Self {
            instruction: instruction.into(),
            input: input.into(),
            max_output_tokens: 512,
            temperature: 0.0,
        }
    }

    pub fn with_max_output_tokens(mut self, max_output_tokens: u32) -> Self {
        self.max_output_tokens = max_output_tokens;
        self
    }

    /// Allow variation. Use for drafting, never for extraction.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }
}

/// What a provider reports about cost.
///
/// Present so a product can show the user what its AI features consumed. A feature
/// that cannot account for itself is one users learn to distrust.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Completion {
    pub text: String,
    pub usage: Usage,
    /// Which model answered. Recorded because the same prompt behaves differently
    /// across models and versions, and "it used to work" needs an answer.
    pub model: String,
    /// `true` when the answer hit `max_output_tokens` and was cut off.
    pub truncated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_prompt_is_deterministic() {
        let prompt = Prompt::new("Summarise", "text");

        assert_eq!(prompt.temperature, 0.0);
        assert_eq!(prompt.max_output_tokens, 512);
    }

    #[test]
    fn temperature_is_clamped_rather_than_trusted() {
        assert_eq!(Prompt::new("x", "y").with_temperature(9.0).temperature, 2.0);
        assert_eq!(
            Prompt::new("x", "y").with_temperature(-1.0).temperature,
            0.0
        );
    }
}
