//! Slack channel adapter.

use crate::channels::domain::{
    Channel, ChannelConfig, ChannelMessage, ChannelStatus, MessageContent, MessageId,
};
use crate::channels::ports::{
    ChannelAdapter, ChannelError, ChannelFeature, MessageFormatter, ReceiveResult, SendResult,
};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU8, Ordering};

/// Slack API adapter.
pub struct SlackAdapter {
    config: ChannelConfig,
    status: AtomicU8,
    client: reqwest::Client,
}

impl SlackAdapter {
    /// Creates a new Slack adapter.
    pub fn new(token: impl Into<String>) -> Self {
        let config = ChannelConfig::new(Channel::Slack)
            .with_token(token)
            .with_base_url("https://slack.com/api");

        Self {
            config,
            status: AtomicU8::new(ChannelStatus::Disconnected as u8),
            client: reqwest::Client::new(),
        }
    }

    /// Creates with custom config.
    pub fn with_config(config: ChannelConfig) -> Self {
        Self {
            config,
            status: AtomicU8::new(ChannelStatus::Disconnected as u8),
            client: reqwest::Client::new(),
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

    fn base_url(&self) -> &str {
        self.config.base_url.as_deref().unwrap_or("https://slack.com/api")
    }

    fn token(&self) -> Result<&str, ChannelError> {
        self.config.token.as_deref().ok_or_else(|| {
            ChannelError::AuthenticationFailed("no token configured".to_string())
        })
    }

    /// Formats message for Slack API.
    fn format_message(&self, message: &ChannelMessage) -> serde_json::Value {
        let text = match &message.content {
            MessageContent::Text(t) => t.clone(),
            MessageContent::RichText(t) => t.clone(),
            MessageContent::Blocks(_) => String::new(),
            MessageContent::Card(_) => String::new(),
            MessageContent::Embed(e) => {
                format!(
                    "*{}*\n{}",
                    e.title.as_deref().unwrap_or(""),
                    e.description.as_deref().unwrap_or("")
                )
            }
        };

        let mut payload = serde_json::json!({
            "channel": message.target,
            "text": text,
        });

        // Add thread_ts if replying
        if let Some(thread) = &message.thread {
            payload["thread_ts"] = serde_json::json!(thread.thread_id);
            if thread.broadcast {
                payload["reply_broadcast"] = serde_json::json!(true);
            }
        }

        // Add blocks if present
        if let MessageContent::Blocks(blocks) = &message.content {
            payload["blocks"] = serde_json::json!(blocks);
        }

        payload
    }
}

#[async_trait]
impl ChannelAdapter for SlackAdapter {
    fn channel(&self) -> Channel {
        Channel::Slack
    }

    fn status(&self) -> ChannelStatus {
        self.get_status()
    }

    async fn connect(&mut self) -> Result<(), ChannelError> {
        self.set_status(ChannelStatus::Connecting);

        // Verify token with auth.test
        let token = self.token()?;
        let response = self.client
            .post(format!("{}/auth.test", self.base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| ChannelError::ConnectionFailed(e.to_string()))?;

        let result: serde_json::Value = response.json().await
            .map_err(|e| ChannelError::SerializationError(e.to_string()))?;

        if result["ok"].as_bool() == Some(true) {
            self.set_status(ChannelStatus::Connected);
            Ok(())
        } else {
            self.set_status(ChannelStatus::Error);
            Err(ChannelError::AuthenticationFailed(
                result["error"].as_str().unwrap_or("unknown").to_string()
            ))
        }
    }

    async fn disconnect(&mut self) -> Result<(), ChannelError> {
        self.set_status(ChannelStatus::Disconnected);
        Ok(())
    }

    async fn send(&self, message: ChannelMessage) -> Result<SendResult, ChannelError> {
        let token = self.token()?;
        let msg_id = message.id.clone();
        let payload = self.format_message(&message);

        let response = self.client
            .post(format!("{}/chat.postMessage", self.base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::ApiError(e.to_string()))?;

        let result: serde_json::Value = response.json().await
            .map_err(|e| ChannelError::SerializationError(e.to_string()))?;

        if result["ok"].as_bool() == Some(true) {
            let ts = result["ts"].as_str().unwrap_or("").to_string();
            Ok(SendResult::success(msg_id, ts))
        } else {
            let error = result["error"].as_str().unwrap_or("unknown").to_string();

            // Check for rate limiting
            if error == "ratelimited" {
                let retry_after = result["retry_after"].as_u64()
                    .map(std::time::Duration::from_secs);
                return Err(ChannelError::RateLimited { retry_after });
            }

            Ok(SendResult::failed(msg_id, error))
        }
    }

    async fn receive(&self, _limit: usize) -> Result<ReceiveResult, ChannelError> {
        // Slack doesn't have a simple "receive all" - would need conversations.history
        // For now, return empty (real impl would use RTM or Events API)
        Ok(ReceiveResult {
            messages: Vec::new(),
            has_more: false,
            cursor: None,
        })
    }

    async fn edit(&self, message_id: &MessageId, new_content: &str) -> Result<(), ChannelError> {
        let token = self.token()?;
        let payload = serde_json::json!({
            "channel": "", // Would need to track channel for message
            "ts": message_id.0,
            "text": new_content,
        });

        let response = self.client
            .post(format!("{}/chat.update", self.base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::ApiError(e.to_string()))?;

        let result: serde_json::Value = response.json().await
            .map_err(|e| ChannelError::SerializationError(e.to_string()))?;

        if result["ok"].as_bool() == Some(true) {
            Ok(())
        } else {
            Err(ChannelError::ApiError(
                result["error"].as_str().unwrap_or("unknown").to_string()
            ))
        }
    }

    async fn delete(&self, message_id: &MessageId) -> Result<(), ChannelError> {
        let token = self.token()?;
        let payload = serde_json::json!({
            "channel": "",
            "ts": message_id.0,
        });

        let response = self.client
            .post(format!("{}/chat.delete", self.base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::ApiError(e.to_string()))?;

        let result: serde_json::Value = response.json().await
            .map_err(|e| ChannelError::SerializationError(e.to_string()))?;

        if result["ok"].as_bool() == Some(true) {
            Ok(())
        } else {
            Err(ChannelError::ApiError(
                result["error"].as_str().unwrap_or("unknown").to_string()
            ))
        }
    }

    async fn react(&self, message_id: &MessageId, emoji: &str) -> Result<(), ChannelError> {
        let token = self.token()?;
        let payload = serde_json::json!({
            "channel": "",
            "timestamp": message_id.0,
            "name": emoji.trim_matches(':'),
        });

        let response = self.client
            .post(format!("{}/reactions.add", self.base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::ApiError(e.to_string()))?;

        let result: serde_json::Value = response.json().await
            .map_err(|e| ChannelError::SerializationError(e.to_string()))?;

        if result["ok"].as_bool() == Some(true) {
            Ok(())
        } else {
            Err(ChannelError::ApiError(
                result["error"].as_str().unwrap_or("unknown").to_string()
            ))
        }
    }
}

/// Slack message formatter.
pub struct SlackFormatter;

impl MessageFormatter for SlackFormatter {
    fn format(&self, message: &ChannelMessage) -> Result<serde_json::Value, ChannelError> {
        let text = match &message.content {
            MessageContent::Text(t) => t.clone(),
            MessageContent::RichText(t) => t.clone(),
            _ => String::new(),
        };

        Ok(serde_json::json!({
            "channel": message.target,
            "text": text,
        }))
    }

    fn parse(&self, raw: &serde_json::Value) -> Result<ChannelMessage, ChannelError> {
        let text = raw["text"].as_str()
            .ok_or_else(|| ChannelError::SerializationError("missing text".to_string()))?;
        let ts = raw["ts"].as_str().unwrap_or("");
        let channel = raw["channel"].as_str().unwrap_or("");

        Ok(ChannelMessage::text(text)
            .to(channel)
            .with_metadata("ts", serde_json::json!(ts)))
    }

    fn max_length(&self) -> usize {
        40000 // Slack's message limit
    }

    fn supports(&self, feature: ChannelFeature) -> bool {
        matches!(feature,
            ChannelFeature::Threading |
            ChannelFeature::Reactions |
            ChannelFeature::Editing |
            ChannelFeature::Deletion |
            ChannelFeature::RichText |
            ChannelFeature::Embeds |
            ChannelFeature::Attachments |
            ChannelFeature::Typing
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slack_adapter_creation() {
        let adapter = SlackAdapter::new("xoxb-test-token");
        assert_eq!(adapter.channel(), Channel::Slack);
        assert_eq!(adapter.status(), ChannelStatus::Disconnected);
    }

    #[test]
    fn test_slack_formatter_max_length() {
        let formatter = SlackFormatter;
        assert_eq!(formatter.max_length(), 40000);
    }

    #[test]
    fn test_slack_formatter_features() {
        let formatter = SlackFormatter;
        assert!(formatter.supports(ChannelFeature::Threading));
        assert!(formatter.supports(ChannelFeature::Reactions));
        assert!(!formatter.supports(ChannelFeature::ReadReceipts));
    }

    #[test]
    fn test_message_formatting() {
        let adapter = SlackAdapter::new("test");
        let msg = ChannelMessage::text("Hello Slack!")
            .to("#general");

        let payload = adapter.format_message(&msg);
        assert_eq!(payload["channel"], "#general");
        assert_eq!(payload["text"], "Hello Slack!");
    }
}
