//! Decision Engine - Multi-LLM orchestration.

use std::sync::Arc;
use anyhow::Result;
use tracing::{info, warn};

use super::provider::{LLMProvider, RawLLMOutput};
use super::context::DecisionContext;
use super::skill::Skill;

/// Operational mode of the DecisionEngine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineMode {
    /// No LLMs available - uses rule-based heuristics only.
    RuleBased,
    /// Single LLM available.
    SingleLLM,
    /// Multiple LLMs - uses consensus voting.
    MultiLLM,
}

/// Result of a decision request.
#[derive(Debug, Clone)]
pub struct Decision {
    /// The recommended action or response
    pub action: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Reasoning behind the decision
    pub reasoning: String,
    /// Which providers contributed
    pub providers_used: Vec<String>,
    /// Tool parameters (if applicable)
    pub parameters: Option<serde_json::Value>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// Decision Engine that orchestrates multi-LLM decisions.
///
/// # Example
///
/// ```rust,no_run
/// use synapse_agentic::decision::{DecisionEngine, DecisionContext};
///
/// #[tokio::main]
/// async fn main() {
///     let engine = DecisionEngine::new();
///
///     let context = DecisionContext::new("operations")
///         .with_summary("Should we scale up the servers?")
///         .with_data(serde_json::json!({
///             "cpu_usage": 85,
///             "memory_usage": 70,
///             "request_queue": 150
///         }));
///
///     let decision = engine.decide(&context).await.unwrap();
///     println!("Decision: {} (confidence: {})", decision.action, decision.confidence);
/// }
/// ```
pub struct DecisionEngine {
    providers: Vec<Arc<dyn LLMProvider>>,
    skills: Vec<Box<dyn Skill>>,
    mode: EngineMode,
}

impl Default for DecisionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DecisionEngine {
    /// Creates a new empty DecisionEngine.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            skills: Vec::new(),
            mode: EngineMode::RuleBased,
        }
    }

    /// Creates a builder for configuring the engine.
    pub fn builder() -> DecisionEngineBuilder {
        DecisionEngineBuilder::new()
    }

    /// Adds a provider to the engine.
    pub fn add_provider(&mut self, provider: impl LLMProvider + 'static) {
        self.providers.push(Arc::new(provider));
        self.update_mode();
    }

    /// Adds a skill to the engine.
    pub fn add_skill(&mut self, skill: impl Skill + 'static) {
        self.skills.push(Box::new(skill));
    }

    /// Returns the current engine mode.
    pub fn mode(&self) -> EngineMode {
        self.mode
    }

    /// Returns the number of providers.
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    fn update_mode(&mut self) {
        self.mode = match self.providers.len() {
            0 => EngineMode::RuleBased,
            1 => EngineMode::SingleLLM,
            _ => EngineMode::MultiLLM,
        };
    }

    /// Makes a decision based on the given context.
    pub async fn decide(&self, context: &DecisionContext) -> Result<Decision> {
        match self.mode {
            EngineMode::RuleBased => self.decide_rule_based(context).await,
            EngineMode::SingleLLM => self.decide_single(context).await,
            EngineMode::MultiLLM => self.decide_consensus(context).await,
        }
    }

    async fn decide_rule_based(&self, context: &DecisionContext) -> Result<Decision> {
        // Simple heuristic-based decision
        info!(domain = %context.domain, "Using rule-based decision (no LLM)");

        Ok(Decision {
            action: "DEFER".to_string(),
            confidence: 0.3,
            reasoning: "No LLM available - using rule-based fallback".to_string(),
            providers_used: vec!["rule_engine".to_string()],
            parameters: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    async fn decide_single(&self, context: &DecisionContext) -> Result<Decision> {
        let provider = &self.providers[0];
        info!(provider = %provider.name(), "Using single LLM for decision");

        let output = provider.generate_decision(context).await?;

        Ok(Decision {
            action: extract_action(&output.text),
            confidence: output.confidence.unwrap_or(0.7),
            reasoning: output.text,
            providers_used: vec![provider.name().to_string()],
            parameters: None, // Will be extracted from output in future refinement
            metadata: std::collections::HashMap::new(),
        })
    }

    async fn decide_consensus(&self, context: &DecisionContext) -> Result<Decision> {
        info!(providers = self.providers.len(), "Using multi-LLM consensus");

        let mut outputs: Vec<(String, RawLLMOutput)> = Vec::new();

        // Query all providers (could be parallelized with tokio::join!)
        for provider in &self.providers {
            match provider.generate_decision(context).await {
                Ok(output) => {
                    outputs.push((provider.name().to_string(), output));
                }
                Err(e) => {
                    warn!(provider = %provider.name(), error = %e, "Provider failed");
                }
            }
        }

        if outputs.is_empty() {
            return self.decide_rule_based(context).await;
        }

        // Simple voting: extract actions and count
        let actions: Vec<String> = outputs.iter().map(|(_, o)| extract_action(&o.text)).collect();
        let most_common = mode_string(&actions);

        let confidence = outputs.iter()
            .filter_map(|(_, o)| o.confidence)
            .sum::<f64>() / outputs.len() as f64;

        let providers_used: Vec<String> = outputs.iter().map(|(n, _)| n.clone()).collect();
        let reasoning = outputs.iter()
            .map(|(n, o)| format!("[{}]: {}", n, o.text.chars().take(200).collect::<String>()))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(Decision {
            action: most_common,
            confidence: confidence.max(0.5),
            reasoning,
            providers_used,
            parameters: None,
            metadata: std::collections::HashMap::new(),
        })
    }
}

/// Builder for DecisionEngine.
pub struct DecisionEngineBuilder {
    providers: Vec<Arc<dyn LLMProvider>>,
    skills: Vec<Box<dyn Skill>>,
}

impl DecisionEngineBuilder {
    /// Creates a new builder.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            skills: Vec::new(),
        }
    }

    /// Adds a provider.
    pub fn with_provider(mut self, provider: impl LLMProvider + 'static) -> Self {
        self.providers.push(Arc::new(provider));
        self
    }

    /// Adds a skill.
    pub fn with_skill(mut self, skill: impl Skill + 'static) -> Self {
        self.skills.push(Box::new(skill));
        self
    }

    /// Builds the engine.
    pub fn build(self) -> DecisionEngine {
        let mode = match self.providers.len() {
            0 => EngineMode::RuleBased,
            1 => EngineMode::SingleLLM,
            _ => EngineMode::MultiLLM,
        };

        DecisionEngine {
            providers: self.providers,
            skills: self.skills,
            mode,
        }
    }
}

impl Default for DecisionEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Helper to extract an action keyword from LLM output
fn extract_action(text: &str) -> String {
    let upper = text.to_uppercase();
    if upper.contains("APPROVE") { return "APPROVE".to_string(); }
    if upper.contains("REJECT") { return "REJECT".to_string(); }
    if upper.contains("YES") { return "YES".to_string(); }
    if upper.contains("NO") { return "NO".to_string(); }
    if upper.contains("PROCEED") { return "PROCEED".to_string(); }
    if upper.contains("WAIT") { return "WAIT".to_string(); }
    "UNKNOWN".to_string()
}

// Helper to find most common string
fn mode_string(items: &[String]) -> String {
    use std::collections::HashMap;
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for item in items {
        *counts.entry(item.as_str()).or_insert(0) += 1;
    }
    counts.into_iter()
        .max_by_key(|(_, c)| *c)
        .map(|(s, _)| s.to_string())
        .unwrap_or_else(|| "UNKNOWN".to_string())
}
