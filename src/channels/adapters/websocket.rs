//! WebSocket channel adapter for generic WS connections.

use crate::channels::domain::{
    Channel, ChannelConfig, ChannelMessage, ChannelStatus, MessageContent, MessageId,
};
use crate::channels::ports::{
    ChannelAdapter, ChannelError, ChannelFeature, MessageFormatter, ReceiveResult, SendResult,
};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

/// WebSocket adapter for generic real-time communication.
pub struct WebSocketAdapter {
    config: ChannelConfig,
    status: AtomicU8,
    message_buffer: Arc<Mutex<VecDeque<ChannelMessage>>>,
    message_counter: Arc<Mutex<u64>>,
}

impl WebSocketAdapter {
    /// Creates a new WebSocket adapter.
    pub fn new(endpoint: impl Into<String>) -> Self {
        let config = ChannelConfig::new(Channel::WebSocket).with_base_url(endpoint);

        Self {
            config,
            status: AtomicU8::new(ChannelStatus::Disconnected as u8),
            message_buffer: Arc::new(Mutex::new(VecDeque::new())),
            message_counter: Arc::new(Mutex::new(0)),
        }
    }

    /// Creates with custom config.
    pub fn with_config(config: ChannelConfig) -> Self {
        Self {
            config,
            status: AtomicU8::new(ChannelStatus::Disconnected as u8),
            message_buffer: Arc::new(Mutex::new(VecDeque::new())),
            message_counter: Arc::new(Mutex::new(0)),
        }
    }

    fn set_status(&self, status: ChannelStatus) {
        self.status.store(status as u8, Ordering::SeqCst);
    }

    fn get_status(&self) -> ChannelStatus {
        match self.status.load(Ordering::SeqCst) {
            0 => ChannelStatus::Disconnected,
            1 => ChannelStatus::Connecting,
            2 => ChannelStatus::Connected,
            3 => ChannelStatus::Reconnecting,
            _ => ChannelStatus::Error,
        }
    }

    fn next_id(&self) -> String {
        let mut counter = self.message_counter.lock().unwrap();
        *counter += 1;
        format!("ws-msg-{}", *counter)
    }

    /// Simulates receiving a message (for testing).
    pub fn inject_message(&self, message: ChannelMessage) {
        if let Ok(mut buffer) = self.message_buffer.lock() {
            buffer.push_back(message);
        }
    }
}

#[async_trait]
impl ChannelAdapter for WebSocketAdapter {
    fn channel(&self) -> Channel {
        Channel::WebSocket
    }

    fn status(&self) -> ChannelStatus {
        self.get_status()
    }

    async fn connect(&mut self) -> Result<(), ChannelError> {
        self.set_status(ChannelStatus::Connecting);

        let url =
            self.config.base_url.as_deref().ok_or_else(|| {
                ChannelError::ConnectionFailed("no endpoint configured".to_string())
            })?;

        // Validate URL format
        if !url.starts_with("ws://") && !url.starts_with("wss://") {
            return Err(ChannelError::ConnectionFailed(
                "invalid WebSocket URL scheme".to_string(),
            ));
        }

        // In a real implementation, this would establish the WebSocket connection
        // For now, we simulate successful connection
        self.set_status(ChannelStatus::Connected);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), ChannelError> {
        self.set_status(ChannelStatus::Disconnected);

        // Clear message buffer on disconnect
        if let Ok(mut buffer) = self.message_buffer.lock() {
            buffer.clear();
        }

        Ok(())
    }

    async fn send(&self, message: ChannelMessage) -> Result<SendResult, ChannelError> {
        if self.get_status() != ChannelStatus::Connected {
            return Err(ChannelError::ConnectionFailed("not connected".to_string()));
        }

        let msg_id = message.id.clone();
        let external_id = self.next_id();

        // In real impl, would serialize and send via WebSocket
        // Simulate successful send
        Ok(SendResult::success(msg_id, external_id))
    }

    async fn receive(&self, limit: usize) -> Result<ReceiveResult, ChannelError> {
        if self.get_status() != ChannelStatus::Connected {
            return Err(ChannelError::ConnectionFailed("not connected".to_string()));
        }

        let mut buffer = self
            .message_buffer
            .lock()
            .map_err(|_| ChannelError::Internal("lock poisoned".to_string()))?;

        let take_count = limit.min(buffer.len());
        let messages: Vec<ChannelMessage> = buffer.drain(..take_count).collect();

        let has_more = !buffer.is_empty();

        Ok(ReceiveResult {
            messages,
            has_more,
            cursor: None,
        })
    }

    async fn edit(&self, _message_id: &MessageId, _new_content: &str) -> Result<(), ChannelError> {
        // WebSocket typically doesn't support editing
        Err(ChannelError::ApiError("edit not supported".to_string()))
    }

    async fn delete(&self, _message_id: &MessageId) -> Result<(), ChannelError> {
        // WebSocket typically doesn't support deletion
        Err(ChannelError::ApiError("delete not supported".to_string()))
    }

    async fn react(&self, _message_id: &MessageId, _emoji: &str) -> Result<(), ChannelError> {
        // WebSocket typically doesn't support reactions
        Err(ChannelError::ApiError("react not supported".to_string()))
    }
}

/// WebSocket message formatter for JSON-based messages.
pub struct WebSocketFormatter {
    /// Optional message type field name.
    pub type_field: String,
    /// Optional payload field name.
    pub payload_field: String,
}

impl Default for WebSocketFormatter {
    fn default() -> Self {
        Self {
            type_field: "type".to_string(),
            payload_field: "data".to_string(),
        }
    }
}

impl WebSocketFormatter {
    /// Creates a new formatter with custom field names.
    pub fn new(type_field: impl Into<String>, payload_field: impl Into<String>) -> Self {
        Self {
            type_field: type_field.into(),
            payload_field: payload_field.into(),
        }
    }
}

impl MessageFormatter for WebSocketFormatter {
    fn format(&self, message: &ChannelMessage) -> Result<serde_json::Value, ChannelError> {
        let content = match &message.content {
            MessageContent::Text(t) => serde_json::json!(t),
            MessageContent::RichText(t) => serde_json::json!(t),
            MessageContent::Blocks(b) => serde_json::json!(b),
            MessageContent::Card(c) => serde_json::json!(c),
            MessageContent::Embed(e) => serde_json::json!({
                "title": e.title,
                "description": e.description,
                "url": e.url,
                "color": e.color,
            }),
        };

        Ok(serde_json::json!({
            self.type_field.clone(): "message",
            self.payload_field.clone(): content,
            "target": message.target,
            "id": message.id.0,
        }))
    }

    fn parse(&self, raw: &serde_json::Value) -> Result<ChannelMessage, ChannelError> {
        let payload = &raw[&self.payload_field];

        let text = if payload.is_string() {
            payload.as_str().unwrap_or("").to_string()
        } else {
            payload.to_string()
        };

        let target = raw["target"].as_str().unwrap_or("");

        Ok(ChannelMessage::text(text).to(target))
    }

    fn max_length(&self) -> usize {
        // WebSocket typically limited by implementation, using common default
        65536 // 64KB
    }

    fn supports(&self, feature: ChannelFeature) -> bool {
        matches!(feature, ChannelFeature::RichText | ChannelFeature::Typing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_adapter_creation() {
        let adapter = WebSocketAdapter::new("wss://example.com/socket");
        assert_eq!(adapter.channel(), Channel::WebSocket);
        assert_eq!(adapter.status(), ChannelStatus::Disconnected);
    }

    #[tokio::test]
    async fn test_websocket_connect() {
        let mut adapter = WebSocketAdapter::new("wss://example.com/socket");
        let result = adapter.connect().await;
        assert!(result.is_ok());
        assert_eq!(adapter.status(), ChannelStatus::Connected);
    }

    #[tokio::test]
    async fn test_websocket_invalid_url() {
        let mut adapter = WebSocketAdapter::new("http://invalid");
        let result = adapter.connect().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_websocket_send_requires_connection() {
        let adapter = WebSocketAdapter::new("wss://example.com");
        let msg = ChannelMessage::text("test");
        let result = adapter.send(msg).await;
        assert!(matches!(result, Err(ChannelError::ConnectionFailed(_))));
    }

    #[tokio::test]
    async fn test_websocket_receive() {
        let mut adapter = WebSocketAdapter::new("wss://example.com");
        adapter.connect().await.unwrap();

        // Inject test message
        adapter.inject_message(ChannelMessage::text("test1"));
        adapter.inject_message(ChannelMessage::text("test2"));

        let result = adapter.receive(10).await.unwrap();
        assert_eq!(result.messages.len(), 2);
        assert!(!result.has_more);
    }

    #[test]
    fn test_websocket_formatter_default() {
        let formatter = WebSocketFormatter::default();
        assert_eq!(formatter.type_field, "type");
        assert_eq!(formatter.payload_field, "data");
    }

    #[test]
    fn test_websocket_formatter_format() {
        let formatter = WebSocketFormatter::default();
        let msg = ChannelMessage::text("Hello WS!").to("room-1");

        let payload = formatter.format(&msg).unwrap();
        assert_eq!(payload["type"], "message");
        assert_eq!(payload["data"], "Hello WS!");
        assert_eq!(payload["target"], "room-1");
    }

    #[test]
    fn test_websocket_formatter_features() {
        let formatter = WebSocketFormatter::default();
        assert!(formatter.supports(ChannelFeature::RichText));
        assert!(!formatter.supports(ChannelFeature::Reactions));
    }
}
