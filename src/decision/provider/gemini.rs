//! Gemini LLM Provider implementation.

use anyhow::Result;
use async_trait::async_trait;
use std::fmt::Debug;
use tracing::{error, info};

use super::LLMProvider;

#[derive(Debug)]
/// Gemini LLM Provider implementation.
pub struct GeminiProvider {
    api_key: String,
    model: String,
}

impl GeminiProvider {
    /// Creates a new GeminiProvider.
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }
}

#[async_trait]
impl LLMProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    fn cost_per_1k_tokens(&self) -> f64 {
        0.001 // Approx
    }

    async fn generate(&self, prompt: &str) -> Result<String> {
        info!("Calling Gemini API (model: {})...", self.model);

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let body = serde_json::json!({
            "contents": [{
                "parts": [{ "text": prompt }]
            }]
        });

        let client = reqwest::Client::new();
        let resp = client.post(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            let err_text = resp.text().await?;
            error!("Gemini API error: {}", err_text);
            return Err(anyhow::anyhow!("Gemini API failed: {}", err_text));
        }

        let json: serde_json::Value = resp.json().await?;

        // Extract text from the response
        let text = json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Gemini response: {:?}", json))?
            .to_string();

        Ok(text)
    }
}
