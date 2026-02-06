//! Simple token estimator using character/word heuristics.

use crate::compaction::domain::Message;
use crate::compaction::ports::{CompactionError, TokenCounter};

/// A simple token estimator that doesn't require external dependencies.
///
/// Uses heuristics based on average token lengths for different models.
/// For production, consider using tiktoken or model-specific tokenizers.
#[derive(Debug, Clone)]
pub struct SimpleTokenEstimator {
    /// Model name for calibration
    model: String,
    /// Average characters per token (varies by model)
    chars_per_token: f32,
}

impl SimpleTokenEstimator {
    /// Creates a new estimator for the given model.
    pub fn new(model: &str) -> Self {
        let chars_per_token = Self::chars_per_token_for_model(model);
        Self {
            model: model.to_string(),
            chars_per_token,
        }
    }

    /// Creates an estimator for GPT-4/Claude models (~4 chars/token).
    pub fn for_gpt4() -> Self {
        Self::new("gpt-4")
    }

    /// Creates an estimator for Claude models.
    pub fn for_claude() -> Self {
        Self::new("claude")
    }

    /// Returns the average characters per token for a model.
    fn chars_per_token_for_model(model: &str) -> f32 {
        let model_lower = model.to_lowercase();

        if model_lower.contains("gpt-4") || model_lower.contains("gpt4") {
            4.0
        } else if model_lower.contains("gpt-3.5") {
            4.0
        } else if model_lower.contains("claude") {
            3.8
        } else if model_lower.contains("gemini") {
            4.2
        } else if model_lower.contains("deepseek") {
            3.5
        } else if model_lower.contains("llama") {
            3.8
        } else {
            // Default conservative estimate
            4.0
        }
    }

    /// Estimates tokens from character count.
    fn estimate_from_chars(&self, char_count: usize) -> u32 {
        ((char_count as f32) / self.chars_per_token).ceil() as u32
    }
}

impl Default for SimpleTokenEstimator {
    fn default() -> Self {
        Self::for_gpt4()
    }
}

impl TokenCounter for SimpleTokenEstimator {
    fn count_tokens(&self, text: &str) -> Result<u32, CompactionError> {
        if text.is_empty() {
            return Ok(0);
        }

        // Primary estimate based on character count
        let char_estimate = self.estimate_from_chars(text.len());

        // Adjust for special cases
        let adjustment = self.calculate_adjustment(text);

        Ok((char_estimate as f32 * adjustment).ceil() as u32)
    }

    fn count_message(&self, message: &Message) -> Result<u32, CompactionError> {
        let content_tokens = self.count_tokens(&message.content)?;

        // Add overhead for message structure
        // Role marker (~2-4 tokens) + potential metadata
        let overhead = match message.role {
            crate::compaction::domain::MessageRole::System => 6,
            crate::compaction::domain::MessageRole::User => 4,
            crate::compaction::domain::MessageRole::Assistant => 4,
            crate::compaction::domain::MessageRole::Tool => 8, // Tool calls have more structure
        };

        Ok(content_tokens + overhead)
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

impl SimpleTokenEstimator {
    /// Calculates adjustment factor based on text characteristics.
    fn calculate_adjustment(&self, text: &str) -> f32 {
        let mut adjustment = 1.0f32;

        // Code tends to have more tokens per character
        if text.contains("```") || text.contains("fn ") || text.contains("def ") {
            adjustment *= 1.15;
        }

        // JSON/structured data has more tokens
        if text.contains("{") && text.contains("}") {
            adjustment *= 1.1;
        }

        // URLs and paths tokenize inefficiently
        let url_count = text.matches("http").count() + text.matches("https").count();
        if url_count > 0 {
            adjustment *= 1.0 + (0.05 * url_count as f32).min(0.2);
        }

        // Numbers often tokenize as multiple tokens
        let digit_ratio = text.chars().filter(|c| c.is_numeric()).count() as f32
            / text.len().max(1) as f32;
        if digit_ratio > 0.1 {
            adjustment *= 1.1;
        }

        adjustment.min(1.5) // Cap adjustment
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_estimation() {
        let estimator = SimpleTokenEstimator::for_gpt4();

        // ~4 chars per token for GPT-4
        let tokens = estimator.count_tokens("Hello, world!").unwrap();
        assert!(tokens >= 3 && tokens <= 5);
    }

    #[test]
    fn test_empty_text() {
        let estimator = SimpleTokenEstimator::default();
        assert_eq!(estimator.count_tokens("").unwrap(), 0);
    }

    #[test]
    fn test_code_adjustment() {
        let estimator = SimpleTokenEstimator::for_gpt4();

        let plain = "This is plain text with forty characters.";
        let code = "```rust\nfn main() { println!(\"Hi\"); }\n```";

        let plain_tokens = estimator.count_tokens(plain).unwrap();
        let code_tokens = estimator.count_tokens(code).unwrap();

        // Code should estimate slightly higher per character
        let plain_ratio = plain_tokens as f32 / plain.len() as f32;
        let code_ratio = code_tokens as f32 / code.len() as f32;

        assert!(code_ratio >= plain_ratio);
    }

    #[test]
    fn test_different_models() {
        let gpt4 = SimpleTokenEstimator::new("gpt-4o");
        let claude = SimpleTokenEstimator::new("claude-sonnet-4");
        let deepseek = SimpleTokenEstimator::new("deepseek-coder");

        let text = "This is a test sentence for token estimation.";

        let gpt4_tokens = gpt4.count_tokens(text).unwrap();
        let claude_tokens = claude.count_tokens(text).unwrap();
        let deepseek_tokens = deepseek.count_tokens(text).unwrap();

        // All should be in reasonable range
        assert!(gpt4_tokens >= 8 && gpt4_tokens <= 15);
        assert!(claude_tokens >= 8 && claude_tokens <= 16);
        assert!(deepseek_tokens >= 9 && deepseek_tokens <= 17);
    }

    #[test]
    fn test_message_overhead() {
        let estimator = SimpleTokenEstimator::default();

        let msg = Message::new(
            crate::compaction::domain::MessageRole::User,
            "Hello"
        );

        let content_tokens = estimator.count_tokens("Hello").unwrap();
        let message_tokens = estimator.count_message(&msg).unwrap();

        // Message should have overhead
        assert!(message_tokens > content_tokens);
    }
}
