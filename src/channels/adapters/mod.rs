//! Adapters for channels module.

mod manager;
mod rate_limiter;
mod slack;
mod websocket;

pub use manager::{ChannelManager, ContentRouter, MessageLogEntry, MessageRouter, RoutingStrategy};
pub use rate_limiter::{CompositeLimiter, SlidingWindowLimiter, TokenBucketLimiter};
pub use slack::{SlackAdapter, SlackFormatter};
pub use websocket::{WebSocketAdapter, WebSocketFormatter};
