pub mod gemini;
pub mod minimax;

pub use gemini::GeminiProvider;
pub use minimax::MinimaxProvider;

use anyhow::Result;
use async_trait::async_trait;
use std::fmt::Debug;

use super::context::DecisionContext;

/// Raw output from an LLM before parsing.
#[derive(Debug, Clone)]
pub struct RawLLMOutput {
    /// The raw text response
    pub text: String,
    /// Confidence score (0.0 - 1.0) if available
    pub confidence: Option<f64>,
    /// Token usage if available
    pub tokens_used: Option<u32>,
}

/// Trait for LLM API integrations.
///
/// Implement this trait to add support for new LLM providers.
///
/// # Example
///
/// ```rust,no_run
/// use synapse_agentic::decision::{LLMProvider, DecisionContext};
/// use async_trait::async_trait;
///
/// #[derive(Debug)]
/// struct MyProvider {
///     api_key: String,
/// }
///
/// #[async_trait]
/// impl LLMProvider for MyProvider {
///     fn name(&self) -> &str { "my-provider" }
///     fn cost_per_1k_tokens(&self) -> f64 { 0.001 }
///
///     async fn generate(&self, prompt: &str) -> anyhow::Result<String> {
///         // Call your LLM API here
///         Ok("Response".to_string())
///     }
/// }
/// ```
#[async_trait]
pub trait LLMProvider: Debug + Send + Sync {
    /// Returns the provider name.
    fn name(&self) -> &str;

    /// Estimated cost per 1000 tokens (for budget tracking).
    fn cost_per_1k_tokens(&self) -> f64;

    /// Generates a response for a simple prompt.
    async fn generate(&self, prompt: &str) -> Result<String>;

    /// Generates a decision based on context.
    ///
    /// Default implementation calls `generate()` with a formatted prompt.
    async fn generate_decision(&self, context: &DecisionContext) -> Result<RawLLMOutput> {
        let prompt = format!(
            "DOMAIN: {}\n\nCONTEXT:\n{}\n\nDATA:\n{}\n\nCONSTRAINTS:\n{}\n\nProvide your decision.",
            context.domain,
            context.summary,
            serde_json::to_string_pretty(&context.data).unwrap_or_default(),
            context.constraints.join("\n")
        );

        let text = self.generate(&prompt).await?;
        Ok(RawLLMOutput {
            text,
            confidence: None,
            tokens_used: None,
        })
    }
}
