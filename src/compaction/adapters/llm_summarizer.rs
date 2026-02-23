//! LLM-based summarization strategy.

use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, warn};

use crate::compaction::domain::{Message, MessageChunk};
use crate::compaction::ports::{CompactionError, SummarizationPrompts, SummarizationStrategy};
use crate::decision::LLMProvider;

/// Summarization strategy that uses an LLM to generate summaries.
pub struct LLMSummarizer {
    /// The LLM provider to use for summarization
    provider: Arc<dyn LLMProvider>,
    /// Custom prompt template (uses default if None)
    custom_prompt: Option<fn(&str) -> String>,
    /// Target compression ratio
    target_ratio: f32,
}

impl LLMSummarizer {
    /// Creates a new LLM summarizer with the given provider.
    pub fn new(provider: Arc<dyn LLMProvider>) -> Self {
        Self {
            provider,
            custom_prompt: None,
            target_ratio: 0.3,
        }
    }

    /// Sets a custom prompt template.
    pub fn with_prompt(mut self, prompt_fn: fn(&str) -> String) -> Self {
        self.custom_prompt = Some(prompt_fn);
        self
    }

    /// Sets the target compression ratio.
    pub fn with_target_ratio(mut self, ratio: f32) -> Self {
        self.target_ratio = ratio.clamp(0.1, 0.9);
        self
    }

    /// Uses technical discussion prompts.
    pub fn for_technical(provider: Arc<dyn LLMProvider>) -> Self {
        Self::new(provider).with_prompt(SummarizationPrompts::technical_prompt)
    }

    /// Uses workflow-focused prompts.
    pub fn for_workflow(provider: Arc<dyn LLMProvider>) -> Self {
        Self::new(provider).with_prompt(SummarizationPrompts::workflow_prompt)
    }

    /// Builds the summarization prompt.
    fn build_prompt(&self, chunk_text: &str) -> String {
        match self.custom_prompt {
            Some(prompt_fn) => prompt_fn(chunk_text),
            None => SummarizationPrompts::default_prompt(chunk_text),
        }
    }
}

#[async_trait]
impl SummarizationStrategy for LLMSummarizer {
    async fn summarize(&self, chunk: &MessageChunk) -> Result<Message, CompactionError> {
        if chunk.is_empty() {
            return Err(CompactionError::NoContent);
        }

        let chunk_text = chunk.to_text();
        let prompt = self.build_prompt(&chunk_text);

        info!(
            provider = %self.provider.name(),
            chunk_messages = chunk.len(),
            chunk_tokens = chunk.total_tokens,
            "Summarizing message chunk"
        );

        let summary_text = self.provider.generate(&prompt).await.map_err(|e| {
            warn!(error = %e, "LLM summarization failed");
            CompactionError::SummarizationFailed(e.to_string())
        })?;

        let replaced_ids = chunk.message_ids();
        let summary = Message::summary(summary_text, replaced_ids);

        info!(
            original_messages = chunk.len(),
            summary_len = summary.content.len(),
            "Chunk summarized successfully"
        );

        Ok(summary)
    }

    fn name(&self) -> &str {
        "llm-summarizer"
    }

    fn target_ratio(&self) -> f32 {
        self.target_ratio
    }
}

/// A mock summarizer for testing (does not require LLM).
#[cfg(test)]
pub struct MockSummarizer {
    prefix: String,
}

#[cfg(test)]
impl MockSummarizer {
    pub fn new() -> Self {
        Self {
            prefix: "SUMMARY:".to_string(),
        }
    }
}

#[cfg(test)]
#[async_trait]
impl SummarizationStrategy for MockSummarizer {
    async fn summarize(&self, chunk: &MessageChunk) -> Result<Message, CompactionError> {
        if chunk.is_empty() {
            return Err(CompactionError::NoContent);
        }
        let summary_text = format!(
            "{} {} messages compacted (tokens: {})",
            self.prefix,
            chunk.len(),
            chunk.total_tokens
        );
        Ok(Message::summary(summary_text, chunk.message_ids()))
    }

    fn name(&self) -> &str {
        "mock-summarizer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compaction::domain::MessageRole;

    fn create_test_chunk() -> MessageChunk {
        let messages = vec![
            Message::new(MessageRole::User, "Hello").with_tokens(10),
            Message::new(MessageRole::Assistant, "Hi there!").with_tokens(15),
            Message::new(MessageRole::User, "How are you?").with_tokens(12),
        ];
        MessageChunk::new(messages, 0)
    }

    #[tokio::test]
    async fn test_mock_summarizer() {
        let summarizer = MockSummarizer::new();
        let chunk = create_test_chunk();

        let result = summarizer.summarize(&chunk).await;
        assert!(result.is_ok());

        let summary = result.unwrap();
        assert!(summary.is_summary);
        assert!(summary.content.contains("3 messages"));
        assert_eq!(summary.summarizes.len(), 3);
    }

    #[tokio::test]
    async fn test_empty_chunk_error() {
        let summarizer = MockSummarizer::new();
        let chunk = MessageChunk::new(vec![], 0);

        let result = summarizer.summarize(&chunk).await;
        assert!(matches!(result, Err(CompactionError::NoContent)));
    }
}
