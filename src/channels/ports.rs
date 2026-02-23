//! Ports Layer: Abstract interfaces for channel operations.

use crate::channels::domain::{Channel, ChannelMessage, ChannelStatus, DeliveryStatus, MessageId};
use async_trait::async_trait;
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur in channel operations.
#[derive(Debug, Error)]
pub enum ChannelError {
    /// Authentication failed
    #[error("authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Connection failed
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    /// Rate limited
    #[error("rate limited, retry after {retry_after:?}")]
    RateLimited {
        /// Duration to wait
        retry_after: Option<Duration>,
    },

    /// Message too large
    #[error("message too large: {size} bytes (max: {max})")]
    MessageTooLarge {
        /// Actual size
        size: usize,
        /// Maximum allowed
        max: usize,
    },

    /// Invalid target
    #[error("invalid target: {0}")]
    InvalidTarget(String),

    /// Timeout
    #[error("operation timed out")]
    Timeout,

    /// Channel not found
    #[error("channel not found: {0}")]
    NotFound(String),

    /// Generic API error
    #[error("API error: {0}")]
    ApiError(String),

    /// Serialization error
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// Internal error
    #[error("internal error: {0}")]
    Internal(String),
}

/// Result of sending a message.
#[derive(Debug)]
pub struct SendResult {
    /// Delivery status
    pub status: DeliveryStatus,
    /// Rate limit remaining
    pub rate_limit_remaining: Option<u32>,
}

impl SendResult {
    /// Creates a successful send result.
    pub fn success(message_id: MessageId, platform_id: impl Into<String>) -> Self {
        Self {
            status: DeliveryStatus::success(message_id, platform_id),
            rate_limit_remaining: None,
        }
    }

    /// Creates a failed send result.
    pub fn failed(message_id: MessageId, error: impl Into<String>) -> Self {
        Self {
            status: DeliveryStatus::failed(message_id, error),
            rate_limit_remaining: None,
        }
    }
}

/// Result of receiving messages.
#[derive(Debug)]
pub struct ReceiveResult {
    /// Received messages
    pub messages: Vec<ChannelMessage>,
    /// Whether there are more messages
    pub has_more: bool,
    /// Cursor for pagination
    pub cursor: Option<String>,
}

/// Port for channel adapters.
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    /// Returns the channel type.
    fn channel(&self) -> Channel;

    /// Returns current connection status.
    fn status(&self) -> ChannelStatus;

    /// Connects to the channel.
    async fn connect(&mut self) -> Result<(), ChannelError>;

    /// Disconnects from the channel.
    async fn disconnect(&mut self) -> Result<(), ChannelError>;

    /// Sends a message.
    async fn send(&self, message: ChannelMessage) -> Result<SendResult, ChannelError>;

    /// Receives messages (polling).
    async fn receive(&self, limit: usize) -> Result<ReceiveResult, ChannelError>;

    /// Edits a previously sent message.
    async fn edit(&self, message_id: &MessageId, new_content: &str) -> Result<(), ChannelError>;

    /// Deletes a message.
    async fn delete(&self, message_id: &MessageId) -> Result<(), ChannelError>;

    /// Adds a reaction to a message.
    async fn react(&self, message_id: &MessageId, emoji: &str) -> Result<(), ChannelError>;
}

/// Port for message formatting.
pub trait MessageFormatter: Send + Sync {
    /// Formats a message for the specific channel.
    fn format(&self, message: &ChannelMessage) -> Result<serde_json::Value, ChannelError>;

    /// Parses a platform-specific message into ChannelMessage.
    fn parse(&self, raw: &serde_json::Value) -> Result<ChannelMessage, ChannelError>;

    /// Maximum message length for this channel.
    fn max_length(&self) -> usize;

    /// Supported features for this channel.
    fn supports(&self, feature: ChannelFeature) -> bool;
}

/// Features that a channel may support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelFeature {
    /// Threading/replies
    Threading,
    /// Reactions/emoji
    Reactions,
    /// Message editing
    Editing,
    /// Message deletion
    Deletion,
    /// Rich formatting (markdown)
    RichText,
    /// Embeds/cards
    Embeds,
    /// File attachments
    Attachments,
    /// Typing indicators
    Typing,
    /// Read receipts
    ReadReceipts,
}

/// Port for rate limiting.
#[async_trait]
pub trait RateLimiter: Send + Sync {
    /// Checks if an operation is allowed without consuming quota.
    async fn check(&self, channel: Channel) -> Result<(), Duration>;

    /// Acquires permission for an operation (consumes quota).
    async fn acquire(&self, channel: Channel) -> Result<(), Duration>;

    /// Releases a previously acquired quota (for cancelled operations).
    async fn release(&self, channel: Channel);

    /// Gets remaining quota for a channel.
    fn remaining(&self, channel: Channel) -> u32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_result() {
        let success = SendResult::success(MessageId::new("msg1"), "platform123");
        assert!(success.status.delivered);

        let failed = SendResult::failed(MessageId::new("msg2"), "error");
        assert!(!failed.status.delivered);
    }

    #[test]
    fn test_channel_error_display() {
        let err = ChannelError::RateLimited {
            retry_after: Some(Duration::from_secs(60)),
        };
        assert!(err.to_string().contains("rate limited"));

        let err = ChannelError::MessageTooLarge {
            size: 5000,
            max: 4000,
        };
        assert!(err.to_string().contains("5000"));
    }
}
