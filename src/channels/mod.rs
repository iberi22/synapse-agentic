//! # Channels Module (Multichannel Adapters)
//!
//! Provides unified communication across multiple platforms.
//!
//! ## Architecture (Hexagonal)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    CHANNELS MODULE                          │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Domain Layer                                               │
//! │  ├── Channel (Slack, Teams, Discord, WebSocket)            │
//! │  ├── Message (normalized cross-channel message)            │
//! │  ├── ThreadContext (conversation threading)                │
//! │  └── Attachment (unified media handling)                   │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Ports Layer                                                │
//! │  ├── ChannelAdapter (main trait for each platform)         │
//! │  ├── MessageFormatter (platform-specific formatting)       │
//! │  └── RateLimiter (throttling per channel)                  │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Adapters Layer                                             │
//! │  ├── SlackAdapter (Slack API integration)                  │
//! │  ├── TeamsAdapter (Microsoft Teams)                        │
//! │  ├── DiscordAdapter (Discord Bot API)                      │
//! │  └── WebSocketAdapter (generic WebSocket)                  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use synapse_agentic::channels::{Channel, ChannelMessage, ChannelManager};
//!
//! let manager = ChannelManager::new();
//! manager.register(SlackAdapter::new("xoxb-token")).await;
//!
//! let msg = ChannelMessage::text("Hello from Synapse!");
//! manager.send(Channel::Slack, "#general", msg).await?;
//! ```

pub mod adapters;
pub mod domain;
pub mod ports;

// Domain exports
pub use domain::{
    Attachment, AttachmentType, Channel, ChannelConfig, ChannelMessage, ChannelStatus,
    DeliveryStatus, EmbedContent, MessageContent, MessageId, ThreadContext,
};

// Port exports
pub use ports::{
    ChannelAdapter, ChannelError, ChannelFeature, MessageFormatter, RateLimiter, ReceiveResult,
    SendResult,
};

// Adapter exports
pub use adapters::{
    ChannelManager, CompositeLimiter, ContentRouter, MessageLogEntry, RoutingStrategy,
    SlackAdapter, SlackFormatter, SlidingWindowLimiter, TokenBucketLimiter, WebSocketAdapter,
    WebSocketFormatter,
};
