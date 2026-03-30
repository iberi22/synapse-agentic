//! Decision context for LLM queries.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Context for decision-making requests.
///
/// This is a domain-agnostic structure that can be used for any business domain.
///
/// # Example
///
/// ```rust
/// use synapse_agentic::decision::DecisionContext;
///
/// let context = DecisionContext::new("hr")
///     .with_summary("Should we approve this vacation request?")
///     .with_data(serde_json::json!({
///         "employee": "John Doe",
///         "days": 5,
///         "start_date": "2026-02-01"
///     }))
///     .with_constraint("Company policy: max 10 consecutive days")
///     .with_constraint("Q1 freeze: no vacations in March");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionContext {
    /// Business domain (e.g., "trading", "hr", "finance", "operations")
    pub domain: String,

    /// Human-readable summary of the situation
    pub summary: String,

    /// Structured data relevant to the decision
    pub data: Value,

    /// Business rules and constraints to consider
    pub constraints: Vec<String>,

    /// Optional: Previous decisions for context
    #[serde(default)]
    pub history: Vec<String>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, Value>,
}

impl DecisionContext {
    /// Creates a new context for the given domain.
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            summary: String::new(),
            data: Value::Null,
            constraints: Vec::new(),
            history: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Sets the summary.
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    /// Sets the structured data.
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = data;
        self
    }

    /// Adds a constraint.
    pub fn with_constraint(mut self, constraint: impl Into<String>) -> Self {
        self.constraints.push(constraint.into());
        self
    }

    /// Adds multiple constraints.
    pub fn with_constraints(
        mut self,
        constraints: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.constraints
            .extend(constraints.into_iter().map(Into::into));
        self
    }

    /// Adds a history entry.
    pub fn with_history(mut self, entry: impl Into<String>) -> Self {
        self.history.push(entry.into());
        self
    }

    /// Adds metadata entry.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}
