//! Adapters for channels module.

mod slack;
mod websocket;
mod rate_limiter;
mod manager;

pub use slack::{SlackAdapter, SlackFormatter};
pub use websocket::{WebSocketAdapter, WebSocketFormatter};
pub use rate_limiter::{TokenBucketLimiter, SlidingWindowLimiter, CompositeLimiter};
pub use manager::{ChannelManager, RoutingStrategy, MessageLogEntry, MessageRouter, ContentRouter};
