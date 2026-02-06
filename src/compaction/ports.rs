//! Application Layer: Ports (Traits) for compaction operations.

use async_trait::async_trait;
use thiserror::Error;

use super::domain::{Message, MessageChunk};

/// Errors that can occur during compaction.
#[derive(Debug, Error)]
pub enum CompactionError {
    /// Summarization failed
    #[error("summarization failed: {0}")]
    SummarizationFailed(String),

    /// Token counting failed
    #[error("token counting failed: {0}")]
    TokenCountFailed(String),

    /// Invalid configuration
    #[error("invalid configuration: {0}")]
    ConfigError(String),

    /// No content to compact
    #[error("no content available for compaction")]
    NoContent,
}

/// Port for counting tokens in text.
pub trait TokenCounter: Send + Sync {
    /// Counts tokens in the given text.
    fn count_tokens(&self, text: &str) -> Result<u32, CompactionError>;

    /// Counts tokens for a message.
    fn count_message(&self, message: &Message) -> Result<u32, CompactionError> {
        // Default: count content + small overhead for role/metadata
        let content_tokens = self.count_tokens(&message.content)?;
        Ok(content_tokens + 4) // ~4 tokens for role markers
    }

    /// Returns the model name this counter is calibrated for.
    fn model_name(&self) -> &str;
}

/// Port for summarizing message chunks.
#[async_trait]
pub trait SummarizationStrategy: Send + Sync {
    /// Summarizes a chunk of messages into a shorter summary.
    async fn summarize(&self, chunk: &MessageChunk) -> Result<Message, CompactionError>;

    /// Returns the name of this strategy.
    fn name(&self) -> &str;

    /// Returns the target compression ratio (e.g., 0.3 = reduce to 30%).
    fn target_ratio(&self) -> f32 {
        0.3
    }
}

/// Prompt templates for summarization.
pub struct SummarizationPrompts;

impl SummarizationPrompts {
    /// Default summarization prompt.
    pub fn default_prompt(chunk_text: &str) -> String {
        format!(
            r#"Summarize the following conversation excerpt concisely while preserving:
1. Key decisions made
2. Open questions or pending tasks
3. Important context and constraints
4. Any action items or next steps

Conversation:
{}

Provide a dense summary that captures the essential information."#,
            chunk_text
        )
    }

    /// Prompt for technical discussions.
    pub fn technical_prompt(chunk_text: &str) -> String {
        format!(
            r#"Summarize this technical discussion, preserving:
1. Technical decisions and rationale
2. Code patterns or architecture discussed
3. Bugs, issues, or blockers mentioned
4. Agreed implementations or TODOs

Discussion:
{}

Provide a technical summary with key points."#,
            chunk_text
        )
    }

    /// Prompt for workflow/BPM contexts.
    pub fn workflow_prompt(chunk_text: &str) -> String {
        format!(
            r#"Summarize this workflow-related conversation:
1. Process steps discussed
2. Approval decisions
3. Blockers or escalations
4. Status changes and outcomes

Conversation:
{}

Provide a workflow-focused summary."#,
            chunk_text
        )
    }
}
