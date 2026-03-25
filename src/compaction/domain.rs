//! Domain Layer: Core compaction entities and value objects.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Role of a message in the conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    /// System instructions
    System,
    /// User input
    User,
    /// Assistant response
    Assistant,
    /// Tool call or result
    Tool,
}

/// A single message in the conversation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier
    pub id: String,
    /// Role of the sender
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Estimated token count (cached)
    pub token_count: Option<u32>,
    /// Whether this is a summary of previous messages
    pub is_summary: bool,
    /// IDs of messages this summary replaces (if is_summary = true)
    pub summarizes: Vec<String>,
}

impl Message {
    /// Creates a new message.
    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role,
            content: content.into(),
            timestamp: SystemTime::now(),
            token_count: None,
            is_summary: false,
            summarizes: Vec::new(),
        }
    }

    /// Creates a summary message that replaces others.
    pub fn summary(content: impl Into<String>, replaced_ids: Vec<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            content: content.into(),
            timestamp: SystemTime::now(),
            token_count: None,
            is_summary: true,
            summarizes: replaced_ids,
        }
    }

    /// Sets the cached token count.
    pub fn with_tokens(mut self, count: u32) -> Self {
        self.token_count = Some(count);
        self
    }
}

/// A chunk of messages that can be summarized together.
#[derive(Debug, Clone)]
pub struct MessageChunk {
    /// Messages in this chunk
    pub messages: Vec<Message>,
    /// Total token count of the chunk
    pub total_tokens: u32,
    /// Start index in original history
    pub start_index: usize,
    /// End index in original history (exclusive)
    pub end_index: usize,
}

impl MessageChunk {
    /// Creates a new chunk from messages.
    pub fn new(messages: Vec<Message>, start_index: usize) -> Self {
        let total_tokens = messages.iter().filter_map(|m| m.token_count).sum();
        let end_index = start_index + messages.len();

        Self {
            messages,
            total_tokens,
            start_index,
            end_index,
        }
    }

    /// Returns the number of messages in this chunk.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Returns true if the chunk is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Extracts all content as a single string for summarization.
    pub fn to_text(&self) -> String {
        self.messages
            .iter()
            .map(|m| format!("[{:?}]: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Returns IDs of all messages in this chunk.
    pub fn message_ids(&self) -> Vec<String> {
        self.messages.iter().map(|m| m.id.clone()).collect()
    }
}

/// Configuration for compaction behavior.
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    /// Maximum tokens before compaction is triggered
    pub hard_limit: u32,
    /// Token count that triggers a warning/preparation
    pub soft_limit: u32,
    /// Minimum messages to keep uncompacted (recent history)
    pub preserve_recent: usize,
    /// Target ratio after compaction (e.g., 0.4 = reduce to 40%)
    pub target_ratio: f32,
    /// Minimum chunk size for summarization
    pub min_chunk_size: usize,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            hard_limit: 100_000, // ~100k tokens
            soft_limit: 80_000,  // Start preparing at 80k
            preserve_recent: 10, // Keep last 10 messages intact
            target_ratio: 0.4,   // Reduce to 40% of original
            min_chunk_size: 5,   // At least 5 messages per chunk
        }
    }
}

impl CompactionConfig {
    /// Creates a config for smaller context windows.
    pub fn small_context() -> Self {
        Self {
            hard_limit: 8_000,
            soft_limit: 6_000,
            preserve_recent: 5,
            target_ratio: 0.3,
            min_chunk_size: 3,
        }
    }

    /// Creates a config for large context windows (128k+).
    pub fn large_context() -> Self {
        Self {
            hard_limit: 120_000,
            soft_limit: 100_000,
            preserve_recent: 20,
            target_ratio: 0.5,
            min_chunk_size: 10,
        }
    }
}

/// Risk level for context overflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextOverflowRisk {
    /// Safe, no action needed
    None,
    /// Approaching limit, prepare for compaction
    Warning,
    /// At limit, compaction required immediately
    Critical,
}

/// The session context containing conversation history.
#[derive(Debug, Clone)]
pub struct SessionContext {
    /// All messages in the session
    pub messages: Vec<Message>,
    /// Configuration for compaction
    pub config: CompactionConfig,
    /// Total token count (cached)
    total_tokens: u32,
}

impl SessionContext {
    /// Creates a new session context.
    pub fn new(config: CompactionConfig) -> Self {
        Self {
            messages: Vec::new(),
            config,
            total_tokens: 0,
        }
    }

    /// Creates a session with default config.
    pub fn default_session() -> Self {
        Self::new(CompactionConfig::default())
    }

    /// Adds a message to the session.
    pub fn add_message(&mut self, message: Message) {
        if let Some(tokens) = message.token_count {
            self.total_tokens += tokens;
        }
        self.messages.push(message);
    }

    /// Returns the total token count.
    pub fn total_tokens(&self) -> u32 {
        self.total_tokens
    }

    /// Recalculates total tokens from messages.
    pub fn recalculate_tokens(&mut self) {
        self.total_tokens = self.messages.iter().filter_map(|m| m.token_count).sum();
    }

    /// Assesses the current overflow risk.
    pub fn overflow_risk(&self) -> ContextOverflowRisk {
        if self.total_tokens >= self.config.hard_limit {
            ContextOverflowRisk::Critical
        } else if self.total_tokens >= self.config.soft_limit {
            ContextOverflowRisk::Warning
        } else {
            ContextOverflowRisk::None
        }
    }

    /// Returns messages that can be compacted (excludes recent).
    pub fn compactable_messages(&self) -> &[Message] {
        let end = self
            .messages
            .len()
            .saturating_sub(self.config.preserve_recent);
        &self.messages[..end]
    }

    /// Returns recent messages that should be preserved.
    pub fn recent_messages(&self) -> &[Message] {
        let start = self
            .messages
            .len()
            .saturating_sub(self.config.preserve_recent);
        &self.messages[start..]
    }

    /// Splits compactable messages into chunks for summarization.
    pub fn create_chunks(&self, max_tokens_per_chunk: u32) -> Vec<MessageChunk> {
        let compactable = self.compactable_messages();
        if compactable.is_empty() {
            return Vec::new();
        }

        let mut chunks = Vec::new();
        let mut current_chunk: Vec<Message> = Vec::new();
        let mut current_tokens = 0u32;
        let mut start_index = 0;

        for (i, message) in compactable.iter().enumerate() {
            let msg_tokens = message.token_count.unwrap_or(100); // Estimate if not cached

            if current_tokens + msg_tokens > max_tokens_per_chunk
                && current_chunk.len() >= self.config.min_chunk_size
            {
                chunks.push(MessageChunk::new(current_chunk, start_index));
                current_chunk = Vec::new();
                current_tokens = 0;
                start_index = i;
            }

            current_chunk.push(message.clone());
            current_tokens += msg_tokens;
        }

        // Don't forget the last chunk
        if current_chunk.len() >= self.config.min_chunk_size {
            chunks.push(MessageChunk::new(current_chunk, start_index));
        }

        chunks
    }

    /// Replaces compacted messages with summaries.
    pub fn apply_compaction(&mut self, result: CompactionResult) {
        // Remove the compacted messages
        let ids_to_remove: std::collections::HashSet<_> =
            result.replaced_message_ids.iter().collect();

        self.messages.retain(|m| !ids_to_remove.contains(&m.id));

        // Insert summaries at the beginning (before recent messages)
        let insert_pos = 0;
        for summary in result.summaries.into_iter().rev() {
            self.messages.insert(insert_pos, summary);
        }

        self.recalculate_tokens();
    }
}

/// Result of a compaction operation.
#[derive(Debug, Clone)]
pub struct CompactionResult {
    /// Generated summary messages
    pub summaries: Vec<Message>,
    /// IDs of messages that were replaced
    pub replaced_message_ids: Vec<String>,
    /// Tokens before compaction
    pub tokens_before: u32,
    /// Tokens after compaction
    pub tokens_after: u32,
    /// Whether compaction was successful
    pub success: bool,
}

impl CompactionResult {
    /// Creates a successful compaction result.
    pub fn success(
        summaries: Vec<Message>,
        replaced_ids: Vec<String>,
        tokens_before: u32,
        tokens_after: u32,
    ) -> Self {
        Self {
            summaries,
            replaced_message_ids: replaced_ids,
            tokens_before,
            tokens_after,
            success: true,
        }
    }

    /// Creates a failed/no-op compaction result.
    pub fn no_compaction(current_tokens: u32) -> Self {
        Self {
            summaries: Vec::new(),
            replaced_message_ids: Vec::new(),
            tokens_before: current_tokens,
            tokens_after: current_tokens,
            success: false,
        }
    }

    /// Returns the compression ratio achieved.
    pub fn compression_ratio(&self) -> f32 {
        if self.tokens_before == 0 {
            1.0
        } else {
            self.tokens_after as f32 / self.tokens_before as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::new(MessageRole::User, "Hello");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello");
        assert!(!msg.is_summary);
    }

    #[test]
    fn test_summary_message() {
        let ids = vec!["id1".to_string(), "id2".to_string()];
        let summary = Message::summary("Summary of conversation", ids.clone());
        assert!(summary.is_summary);
        assert_eq!(summary.summarizes, ids);
    }

    #[test]
    fn test_session_overflow_risk() {
        let config = CompactionConfig {
            hard_limit: 100,
            soft_limit: 80,
            ..Default::default()
        };
        let mut session = SessionContext::new(config);

        assert_eq!(session.overflow_risk(), ContextOverflowRisk::None);

        session.add_message(Message::new(MessageRole::User, "test").with_tokens(85));
        assert_eq!(session.overflow_risk(), ContextOverflowRisk::Warning);

        session.add_message(Message::new(MessageRole::User, "test").with_tokens(20));
        assert_eq!(session.overflow_risk(), ContextOverflowRisk::Critical);
    }

    #[test]
    fn test_chunk_creation() {
        let config = CompactionConfig {
            preserve_recent: 2,
            min_chunk_size: 2,
            ..Default::default()
        };
        let mut session = SessionContext::new(config);

        for i in 0..10 {
            session.add_message(
                Message::new(MessageRole::User, format!("Message {}", i)).with_tokens(100),
            );
        }

        let chunks = session.create_chunks(250);
        assert!(!chunks.is_empty());

        // Recent 2 should be preserved
        assert_eq!(session.recent_messages().len(), 2);
    }
}
