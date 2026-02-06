//! Skills - Reusable AI capabilities.

use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::fmt::Debug;
use anyhow::Result;

use super::provider::LLMProvider;
use super::context::DecisionContext;

/// Output from a skill execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    /// Name of the skill that produced this output
    pub skill_name: String,

    /// Type of output (e.g., "analysis", "action", "risk_check")
    pub output_type: String,

    /// The actual result as JSON
    pub result: serde_json::Value,

    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
}

/// Trait for reusable AI capabilities.
///
/// Skills are modular units of AI functionality that can be composed
/// and reused across different agents and workflows.
///
/// # Example
///
/// ```rust,no_run
/// use synapse_agentic::decision::{Skill, SkillOutput, DecisionContext, LLMProvider};
/// use async_trait::async_trait;
///
/// #[derive(Debug)]
/// struct SentimentSkill;
///
/// #[async_trait]
/// impl Skill for SentimentSkill {
///     fn name(&self) -> &str { "sentiment_analysis" }
///     fn description(&self) -> &str { "Analyzes sentiment of text" }
///
///     async fn execute(
///         &self,
///         context: &DecisionContext,
///         provider: &dyn LLMProvider,
///     ) -> anyhow::Result<SkillOutput> {
///         let prompt = format!(
///             "Analyze the sentiment of this text and respond with JSON:\n{}",
///             context.summary
///         );
///
///         let response = provider.generate(&prompt).await?;
///
///         Ok(SkillOutput {
///             skill_name: self.name().to_string(),
///             output_type: "analysis".to_string(),
///             result: serde_json::json!({ "raw": response }),
///             confidence: 0.8,
///         })
///     }
/// }
/// ```
#[async_trait]
pub trait Skill: Debug + Send + Sync {
    /// Returns the skill name.
    fn name(&self) -> &str;

    /// Returns a description of what the skill does.
    fn description(&self) -> &str;

    /// Executes the skill with the given context and LLM provider.
    async fn execute(
        &self,
        context: &DecisionContext,
        provider: &dyn LLMProvider,
    ) -> Result<SkillOutput>;
}

/// A skill defined by a text prompt template.
///
/// Useful for quickly creating skills without implementing the trait.
#[allow(dead_code)]
#[derive(Debug)]
pub struct PromptSkill {
    name: String,
    description: String,
    prompt_template: String,
}

impl PromptSkill {
    /// Creates a new prompt-based skill.
    #[allow(dead_code)]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        prompt_template: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            prompt_template: prompt_template.into(),
        }
    }
}

#[async_trait]
impl Skill for PromptSkill {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    async fn execute(
        &self,
        context: &DecisionContext,
        provider: &dyn LLMProvider,
    ) -> Result<SkillOutput> {
        let prompt = self.prompt_template
            .replace("{domain}", &context.domain)
            .replace("{summary}", &context.summary)
            .replace("{data}", &serde_json::to_string_pretty(&context.data).unwrap_or_default());

        let response = provider.generate(&prompt).await?;

        // Try to parse as JSON, otherwise wrap as string
        let result = serde_json::from_str(&response)
            .unwrap_or_else(|_| serde_json::json!({ "text": response }));

        Ok(SkillOutput {
            skill_name: self.name.clone(),
            output_type: "analysis".to_string(),
            result,
            confidence: 0.7,
        })
    }
}
