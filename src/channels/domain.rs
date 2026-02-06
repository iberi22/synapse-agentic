//! Domain Layer: Channel entities and value objects.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Supported communication channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Channel {
    /// Slack workspace
    Slack,
    /// Microsoft Teams
    Teams,
    /// Discord server
    Discord,
    /// Telegram bot
    Telegram,
    /// Generic WebSocket connection
    WebSocket,
    /// HTTP webhook
    Webhook,
    /// Email (SMTP)
    Email,
    /// Custom channel
    Custom,
}

impl Channel {
    /// Returns a human-readable name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Channel::Slack => "Slack",
            Channel::Teams => "Microsoft Teams",
            Channel::Discord => "Discord",
            Channel::Telegram => "Telegram",
            Channel::WebSocket => "WebSocket",
            Channel::Webhook => "Webhook",
            Channel::Email => "Email",
            Channel::Custom => "Custom",
        }
    }

    /// Returns default rate limit (messages per minute).
    pub fn default_rate_limit(&self) -> u32 {
        match self {
            Channel::Slack => 50,      // Tier 3 rate limit
            Channel::Teams => 30,      // Conservative
            Channel::Discord => 50,    // Per channel
            Channel::Telegram => 30,   // Per chat
            Channel::WebSocket => 100, // Usually unlimited
            Channel::Webhook => 60,    // Conservative
            Channel::Email => 10,      // Very conservative
            Channel::Custom => 60,
        }
    }
}

/// Unique identifier for a message.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub String);

impl MessageId {
    /// Creates a new message ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generates a new unique ID.
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Content of a channel message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    /// Plain text
    Text(String),
    /// Rich text with formatting (markdown-like)
    RichText(String),
    /// Structured blocks (Slack Block Kit style)
    Blocks(Vec<serde_json::Value>),
    /// Card/adaptive card (Teams style)
    Card(serde_json::Value),
    /// Embedded content
    Embed(EmbedContent),
}

/// Embedded content in a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedContent {
    /// Title of the embed
    pub title: Option<String>,
    /// Description/body
    pub description: Option<String>,
    /// URL to link
    pub url: Option<String>,
    /// Color (hex)
    pub color: Option<String>,
    /// Fields (key-value pairs)
    pub fields: Vec<EmbedField>,
    /// Footer text
    pub footer: Option<String>,
    /// Thumbnail URL
    pub thumbnail: Option<String>,
    /// Timestamp
    pub timestamp: Option<DateTime<Utc>>,
}

impl EmbedContent {
    /// Creates a new embed.
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            url: None,
            color: None,
            fields: Vec::new(),
            footer: None,
            thumbnail: None,
            timestamp: None,
        }
    }

    /// Sets the title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Adds a field.
    pub fn with_field(mut self, name: impl Into<String>, value: impl Into<String>, inline: bool) -> Self {
        self.fields.push(EmbedField {
            name: name.into(),
            value: value.into(),
            inline,
        });
        self
    }
}

impl Default for EmbedContent {
    fn default() -> Self {
        Self::new()
    }
}

/// Field in an embed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedField {
    /// Field name/title
    pub name: String,
    /// Field value
    pub value: String,
    /// Whether to display inline
    pub inline: bool,
}

/// Normalized cross-channel message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    /// Unique message ID
    pub id: MessageId,
    /// Message content
    pub content: MessageContent,
    /// Target channel/conversation
    pub target: String,
    /// Thread context (for replies)
    pub thread: Option<ThreadContext>,
    /// Attachments
    pub attachments: Vec<Attachment>,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl ChannelMessage {
    /// Creates a simple text message.
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            id: MessageId::generate(),
            content: MessageContent::Text(content.into()),
            target: String::new(),
            thread: None,
            attachments: Vec::new(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Creates a rich text message.
    pub fn rich(content: impl Into<String>) -> Self {
        Self {
            id: MessageId::generate(),
            content: MessageContent::RichText(content.into()),
            target: String::new(),
            thread: None,
            attachments: Vec::new(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Creates an embed message.
    pub fn embed(embed: EmbedContent) -> Self {
        Self {
            id: MessageId::generate(),
            content: MessageContent::Embed(embed),
            target: String::new(),
            thread: None,
            attachments: Vec::new(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Sets the target channel/conversation.
    pub fn to(mut self, target: impl Into<String>) -> Self {
        self.target = target.into();
        self
    }

    /// Sets thread context for reply.
    pub fn in_thread(mut self, thread: ThreadContext) -> Self {
        self.thread = Some(thread);
        self
    }

    /// Adds an attachment.
    pub fn with_attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Adds metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Thread context for conversation threading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadContext {
    /// Parent message ID (thread root)
    pub parent_id: MessageId,
    /// Thread ID (platform-specific)
    pub thread_id: String,
    /// Whether to broadcast to channel
    pub broadcast: bool,
}

impl ThreadContext {
    /// Creates a new thread context.
    pub fn new(parent_id: MessageId, thread_id: impl Into<String>) -> Self {
        Self {
            parent_id,
            thread_id: thread_id.into(),
            broadcast: false,
        }
    }

    /// Sets broadcast flag.
    pub fn with_broadcast(mut self, broadcast: bool) -> Self {
        self.broadcast = broadcast;
        self
    }
}

/// Type of attachment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttachmentType {
    /// Image file
    Image,
    /// Video file
    Video,
    /// Audio file
    Audio,
    /// Generic file
    File,
    /// Code snippet
    Code,
}

/// File/media attachment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// Attachment type
    pub attachment_type: AttachmentType,
    /// Filename
    pub filename: String,
    /// MIME type
    pub mime_type: Option<String>,
    /// URL (if hosted)
    pub url: Option<String>,
    /// Raw content (if inline)
    pub content: Option<Vec<u8>>,
    /// Size in bytes
    pub size: Option<usize>,
    /// Alt text for images
    pub alt_text: Option<String>,
}

impl Attachment {
    /// Creates an image attachment from URL.
    pub fn image_url(url: impl Into<String>, filename: impl Into<String>) -> Self {
        Self {
            attachment_type: AttachmentType::Image,
            filename: filename.into(),
            mime_type: None,
            url: Some(url.into()),
            content: None,
            size: None,
            alt_text: None,
        }
    }

    /// Creates a file attachment.
    pub fn file(filename: impl Into<String>, content: Vec<u8>, mime_type: impl Into<String>) -> Self {
        let content_len = content.len();
        Self {
            attachment_type: AttachmentType::File,
            filename: filename.into(),
            mime_type: Some(mime_type.into()),
            url: None,
            content: Some(content),
            size: Some(content_len),
            alt_text: None,
        }
    }

    /// Creates a code snippet attachment.
    pub fn code(filename: impl Into<String>, code: impl Into<String>, language: Option<&str>) -> Self {
        let code_str = code.into();
        let code_bytes = code_str.into_bytes();
        let size = code_bytes.len();
        Self {
            attachment_type: AttachmentType::Code,
            filename: filename.into(),
            mime_type: language.map(|l| format!("text/x-{}", l)),
            url: None,
            content: Some(code_bytes),
            size: Some(size),
            alt_text: None,
        }
    }
}

/// Configuration for a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// Channel type
    pub channel: Channel,
    /// Authentication token/credentials
    pub token: Option<String>,
    /// API base URL (for custom endpoints)
    pub base_url: Option<String>,
    /// Rate limit override (messages per minute)
    pub rate_limit: Option<u32>,
    /// Connection timeout (seconds)
    pub timeout_secs: u64,
    /// Auto-reconnect on disconnect
    pub auto_reconnect: bool,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Custom headers
    pub headers: HashMap<String, String>,
}

impl ChannelConfig {
    /// Creates a new config for a channel.
    pub fn new(channel: Channel) -> Self {
        Self {
            channel,
            token: None,
            base_url: None,
            rate_limit: None,
            timeout_secs: 30,
            auto_reconnect: true,
            max_retries: 3,
            headers: HashMap::new(),
        }
    }

    /// Sets the authentication token.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Sets the base URL.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Sets a custom rate limit.
    pub fn with_rate_limit(mut self, limit: u32) -> Self {
        self.rate_limit = Some(limit);
        self
    }

    /// Gets the effective rate limit.
    pub fn effective_rate_limit(&self) -> u32 {
        self.rate_limit.unwrap_or_else(|| self.channel.default_rate_limit())
    }
}

/// Status of a channel connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelStatus {
    /// Not connected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected and ready
    Connected,
    /// Reconnecting after disconnect
    Reconnecting,
    /// Error state
    Error,
}

/// Status of message delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryStatus {
    /// Message ID
    pub message_id: MessageId,
    /// Whether delivery succeeded
    pub delivered: bool,
    /// Platform-specific message ID (if delivered)
    pub platform_id: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Delivery timestamp
    pub timestamp: DateTime<Utc>,
}

impl DeliveryStatus {
    /// Creates a successful delivery status.
    pub fn success(message_id: MessageId, platform_id: impl Into<String>) -> Self {
        Self {
            message_id,
            delivered: true,
            platform_id: Some(platform_id.into()),
            error: None,
            timestamp: Utc::now(),
        }
    }

    /// Creates a failed delivery status.
    pub fn failed(message_id: MessageId, error: impl Into<String>) -> Self {
        Self {
            message_id,
            delivered: false,
            platform_id: None,
            error: Some(error.into()),
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_rate_limits() {
        assert_eq!(Channel::Slack.default_rate_limit(), 50);
        assert_eq!(Channel::Email.default_rate_limit(), 10);
    }

    #[test]
    fn test_message_builder() {
        let msg = ChannelMessage::text("Hello")
            .to("#general")
            .with_metadata("priority", serde_json::json!("high"));

        assert_eq!(msg.target, "#general");
        assert!(msg.metadata.contains_key("priority"));
    }

    #[test]
    fn test_embed_builder() {
        let embed = EmbedContent::new()
            .with_title("Alert")
            .with_description("Something happened")
            .with_field("Status", "Active", true);

        assert_eq!(embed.title, Some("Alert".to_string()));
        assert_eq!(embed.fields.len(), 1);
    }

    #[test]
    fn test_thread_context() {
        let ctx = ThreadContext::new(MessageId::new("parent123"), "thread456")
            .with_broadcast(true);

        assert!(ctx.broadcast);
        assert_eq!(ctx.thread_id, "thread456");
    }

    #[test]
    fn test_attachment_code() {
        let attachment = Attachment::code("main.rs", "fn main() {}", Some("rust"));

        assert_eq!(attachment.attachment_type, AttachmentType::Code);
        assert!(attachment.content.is_some());
    }

    #[test]
    fn test_channel_config() {
        let config = ChannelConfig::new(Channel::Slack)
            .with_token("xoxb-test")
            .with_rate_limit(100);

        assert_eq!(config.effective_rate_limit(), 100);
        assert_eq!(config.channel, Channel::Slack);
    }

    #[test]
    fn test_delivery_status() {
        let success = DeliveryStatus::success(MessageId::new("msg1"), "ts123");
        assert!(success.delivered);

        let failed = DeliveryStatus::failed(MessageId::new("msg2"), "timeout");
        assert!(!failed.delivered);
        assert!(failed.error.is_some());
    }
}
