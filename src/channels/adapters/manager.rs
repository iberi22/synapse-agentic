//! Channel manager for coordinating multiple channel adapters.

use crate::channels::domain::{Channel, ChannelMessage, ChannelStatus, DeliveryStatus, MessageId};
use crate::channels::ports::{
    ChannelAdapter, ChannelError, RateLimiter, ReceiveResult, SendResult,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Manages multiple channel adapters with routing and failover.
pub struct ChannelManager {
    adapters: RwLock<HashMap<Channel, Arc<RwLock<dyn ChannelAdapter + Send + Sync>>>>,
    rate_limiter: Option<Arc<dyn RateLimiter + Send + Sync>>,
    routing_strategy: RoutingStrategy,
    message_log: RwLock<Vec<MessageLogEntry>>,
}

/// Strategy for routing messages to channels.
#[derive(Debug, Clone, Default)]
pub enum RoutingStrategy {
    /// Send to a single channel.
    #[default]
    Direct,
    /// Send to all registered channels.
    Broadcast,
    /// Send to multiple specified channels.
    Multicast(Vec<Channel>),
    /// Route based on message target prefix.
    PrefixBased(HashMap<String, Channel>),
}

/// Log entry for sent messages.
#[derive(Debug, Clone)]
pub struct MessageLogEntry {
    /// Unique identifier for the message.
    pub message_id: MessageId,
    /// Channel through which the message was sent.
    pub channel: Channel,
    /// Timestamp when the message was sent.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Delivery status of the message.
    pub status: DeliveryStatus,
    /// External identifier from the channel provider, if available.
    pub external_id: Option<String>,
}

impl ChannelManager {
    /// Creates a new channel manager.
    pub fn new() -> Self {
        Self {
            adapters: RwLock::new(HashMap::new()),
            rate_limiter: None,
            routing_strategy: RoutingStrategy::Direct,
            message_log: RwLock::new(Vec::new()),
        }
    }

    /// Sets the routing strategy.
    pub fn with_strategy(mut self, strategy: RoutingStrategy) -> Self {
        self.routing_strategy = strategy;
        self
    }

    /// Sets a global rate limiter.
    pub fn with_rate_limiter<R: RateLimiter + Send + Sync + 'static>(
        mut self,
        limiter: R,
    ) -> Self {
        self.rate_limiter = Some(Arc::new(limiter));
        self
    }

    /// Registers a channel adapter.
    pub async fn register<A: ChannelAdapter + Send + Sync + 'static>(
        &self,
        adapter: A,
    ) {
        let channel = adapter.channel();
        let mut adapters = self.adapters.write().await;
        adapters.insert(channel, Arc::new(RwLock::new(adapter)));
    }

    /// Unregisters a channel adapter.
    pub async fn unregister(&self, channel: Channel) -> Option<()> {
        let mut adapters = self.adapters.write().await;
        adapters.remove(&channel).map(|_| ())
    }

    /// Gets the status of a specific channel.
    pub async fn channel_status(&self, channel: Channel) -> Option<ChannelStatus> {
        let adapters = self.adapters.read().await;
        if let Some(adapter) = adapters.get(&channel) {
            let adapter_guard = adapter.read().await;
            Some(adapter_guard.status())
        } else {
            None
        }
    }

    /// Gets all registered channels.
    pub async fn channels(&self) -> Vec<Channel> {
        let adapters = self.adapters.read().await;
        adapters.keys().cloned().collect()
    }

    /// Connects all registered adapters.
    pub async fn connect_all(&self) -> HashMap<Channel, Result<(), ChannelError>> {
        let adapters = self.adapters.read().await;
        let mut results = HashMap::new();

        for (channel, adapter) in adapters.iter() {
            let mut adapter_guard = adapter.write().await;
            results.insert(*channel, adapter_guard.connect().await);
        }

        results
    }

    /// Disconnects all registered adapters.
    pub async fn disconnect_all(&self) -> HashMap<Channel, Result<(), ChannelError>> {
        let adapters = self.adapters.read().await;
        let mut results = HashMap::new();

        for (channel, adapter) in adapters.iter() {
            let mut adapter_guard = adapter.write().await;
            results.insert(*channel, adapter_guard.disconnect().await);
        }

        results
    }

    /// Sends a message to the specified channel.
    pub async fn send_to(
        &self,
        channel: Channel,
        message: ChannelMessage,
    ) -> Result<SendResult, ChannelError> {
        // Check rate limit if configured
        if let Some(ref limiter) = self.rate_limiter {
            if let Err(wait) = limiter.acquire(channel).await {
                return Err(ChannelError::RateLimited {
                    retry_after: Some(wait),
                });
            }
        }

        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&channel)
            .ok_or(ChannelError::NotFound(format!("{:?}", channel)))?;

        let adapter_guard = adapter.read().await;
        let result = adapter_guard.send(message.clone()).await?;

        // Log the message
        self.log_message(&message.id, channel, &result).await;

        Ok(result)
    }

    /// Sends a message using the configured routing strategy.
    pub async fn send(&self, message: ChannelMessage) -> Vec<(Channel, Result<SendResult, ChannelError>)> {
        let channels = self.resolve_channels(&message).await;
        let mut results = Vec::new();

        for channel in channels {
            let result = self.send_to(channel, message.clone()).await;
            results.push((channel, result));
        }

        results
    }

    /// Broadcasts a message to all connected channels.
    pub async fn broadcast(&self, message: ChannelMessage) -> Vec<(Channel, Result<SendResult, ChannelError>)> {
        let adapters = self.adapters.read().await;
        let mut results = Vec::new();

        for (&channel, adapter) in adapters.iter() {
            let adapter_guard = adapter.read().await;
            if adapter_guard.status() == ChannelStatus::Connected {
                let result = adapter_guard.send(message.clone()).await;
                if let Ok(ref send_result) = result {
                    self.log_message(&message.id, channel, send_result).await;
                }
                results.push((channel, result));
            }
        }

        results
    }

    /// Receives messages from a specific channel.
    pub async fn receive_from(
        &self,
        channel: Channel,
        limit: usize,
    ) -> Result<ReceiveResult, ChannelError> {
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&channel)
            .ok_or(ChannelError::NotFound(format!("{:?}", channel)))?;

        let adapter_guard = adapter.read().await;
        adapter_guard.receive(limit).await
    }

    /// Receives messages from all connected channels.
    pub async fn receive_all(&self, limit: usize) -> HashMap<Channel, Result<ReceiveResult, ChannelError>> {
        let adapters = self.adapters.read().await;
        let mut results = HashMap::new();

        for (&channel, adapter) in adapters.iter() {
            let adapter_guard = adapter.read().await;
            if adapter_guard.status() == ChannelStatus::Connected {
                results.insert(channel, adapter_guard.receive(limit).await);
            }
        }

        results
    }

    /// Gets the message log.
    pub async fn message_log(&self) -> Vec<MessageLogEntry> {
        let log = self.message_log.read().await;
        log.clone()
    }

    /// Gets log entries for a specific message.
    pub async fn message_status(&self, message_id: &MessageId) -> Vec<MessageLogEntry> {
        let log = self.message_log.read().await;
        log.iter()
            .filter(|e| &e.message_id == message_id)
            .cloned()
            .collect()
    }

    /// Clears the message log.
    pub async fn clear_log(&self) {
        let mut log = self.message_log.write().await;
        log.clear();
    }

    async fn resolve_channels(&self, message: &ChannelMessage) -> Vec<Channel> {
        match &self.routing_strategy {
            RoutingStrategy::Direct => {
                // Use first available channel
                let adapters = self.adapters.read().await;
                adapters.keys().next().cloned().into_iter().collect()
            }
            RoutingStrategy::Broadcast => {
                let adapters = self.adapters.read().await;
                adapters.keys().cloned().collect()
            }
            RoutingStrategy::Multicast(channels) => channels.clone(),
            RoutingStrategy::PrefixBased(prefixes) => {
                for (prefix, channel) in prefixes {
                    if message.target.starts_with(prefix) {
                        return vec![*channel];
                    }
                }
                Vec::new()
            }
        }
    }

    async fn log_message(&self, message_id: &MessageId, channel: Channel, result: &SendResult) {
        let entry = MessageLogEntry {
            message_id: message_id.clone(),
            channel,
            timestamp: chrono::Utc::now(),
            status: result.status.clone(),
            external_id: result.status.platform_id.clone(),
        };

        let mut log = self.message_log.write().await;
        log.push(entry);
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for message routing.
#[async_trait]
pub trait MessageRouter: Send + Sync {
    /// Routes a message to appropriate channels.
    async fn route(&self, message: &ChannelMessage) -> Vec<Channel>;
}

/// Content-based router.
pub struct ContentRouter {
    rules: Vec<(Box<dyn Fn(&ChannelMessage) -> bool + Send + Sync>, Channel)>,
}

impl ContentRouter {
    /// Creates a new content router.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Adds a routing rule.
    pub fn add_rule<F>(mut self, predicate: F, channel: Channel) -> Self
    where
        F: Fn(&ChannelMessage) -> bool + Send + Sync + 'static,
    {
        self.rules.push((Box::new(predicate), channel));
        self
    }
}

impl Default for ContentRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageRouter for ContentRouter {
    async fn route(&self, message: &ChannelMessage) -> Vec<Channel> {
        self.rules
            .iter()
            .filter_map(|(pred, channel)| {
                if pred(message) {
                    Some(*channel)
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::adapters::WebSocketAdapter;

    #[tokio::test]
    async fn test_channel_manager_creation() {
        let manager = ChannelManager::new();
        assert!(manager.channels().await.is_empty());
    }

    #[tokio::test]
    async fn test_register_adapter() {
        let manager = ChannelManager::new();
        let adapter = WebSocketAdapter::new("wss://test.com");

        manager.register(adapter).await;

        let channels = manager.channels().await;
        assert!(channels.contains(&Channel::WebSocket));
    }

    #[tokio::test]
    async fn test_unregister_adapter() {
        let manager = ChannelManager::new();
        let adapter = WebSocketAdapter::new("wss://test.com");

        manager.register(adapter).await;
        assert!(manager.unregister(Channel::WebSocket).await.is_some());
        assert!(manager.channels().await.is_empty());
    }

    #[tokio::test]
    async fn test_channel_status() {
        let manager = ChannelManager::new();
        let adapter = WebSocketAdapter::new("wss://test.com");

        manager.register(adapter).await;

        let status = manager.channel_status(Channel::WebSocket).await;
        assert_eq!(status, Some(ChannelStatus::Disconnected));
    }

    #[tokio::test]
    async fn test_send_to_unknown_channel() {
        let manager = ChannelManager::new();
        let msg = ChannelMessage::text("test");

        let result = manager.send_to(Channel::Slack, msg).await;
        assert!(matches!(result, Err(ChannelError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_routing_strategy_broadcast() {
        let manager = ChannelManager::new()
            .with_strategy(RoutingStrategy::Broadcast);

        let adapter1 = WebSocketAdapter::new("wss://test1.com");
        let adapter2 = WebSocketAdapter::new("wss://test2.com");

        manager.register(adapter1).await;
        // Note: Can't register two of same type, so just test strategy logic

        let msg = ChannelMessage::text("broadcast");
        let results = manager.send(msg).await;

        // Should attempt to send to registered channel(s)
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_content_router() {
        let router = ContentRouter::new()
            .add_rule(|m| m.target.starts_with("#"), Channel::Slack)
            .add_rule(|m| m.target.starts_with("@"), Channel::Discord);

        let slack_msg = ChannelMessage::text("test").to("#general");
        let discord_msg = ChannelMessage::text("test").to("@user");

        let slack_routes = router.route(&slack_msg).await;
        let discord_routes = router.route(&discord_msg).await;

        assert!(slack_routes.contains(&Channel::Slack));
        assert!(discord_routes.contains(&Channel::Discord));
    }

    #[tokio::test]
    async fn test_message_log() {
        let manager = ChannelManager::new();
        let adapter = WebSocketAdapter::new("wss://test.com");

        manager.register(adapter).await;

        // Connect first
        manager.connect_all().await;

        // Log should be empty initially
        assert!(manager.message_log().await.is_empty());
    }

    #[test]
    fn test_routing_strategy_default() {
        let strategy = RoutingStrategy::default();
        assert!(matches!(strategy, RoutingStrategy::Direct));
    }
}
