//! Minimax LLM Provider implementation.

use async_trait::async_trait;
use anyhow::Result;
use tracing::{info, error};
use std::fmt::Debug;

use super::LLMProvider;

#[derive(Debug)]
/// Minimax LLM Provider implementation.
pub struct MinimaxProvider {
    api_key: String,
    _group_id: String,
    model: String,
}

impl MinimaxProvider {
    /// Creates a new MinimaxProvider.
    pub fn new(api_key: String, group_id: String, model: String) -> Self {
        Self { api_key, _group_id: group_id, model }
    }
}

#[async_trait]
impl LLMProvider for MinimaxProvider {
    fn name(&self) -> &str {
        "minimax"
    }

    fn cost_per_1k_tokens(&self) -> f64 {
        0.001
    }

    async fn generate(&self, prompt: &str) -> Result<String> {
        info!("Calling Minimax API (model: {})...", self.model);

        let url = "https://api.minimax.chat/v1/text/chatcompletion_v2";

        let body = serde_json::json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": prompt
            }]
        });

        let client = reqwest::Client::new();
        let resp = client.post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err_text = resp.text().await?;
            error!("Minimax API error: {}", err_text);
            return Err(anyhow::anyhow!("Minimax API failed: {}", err_text));
        }

        let json: serde_json::Value = resp.json().await?;

        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Minimax response: {:?}", json))?
            .to_string();

        Ok(text)
    }
}
