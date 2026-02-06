//! Ports Layer: Abstract interfaces for parsing operations.

use crate::parser::domain::{LLMOutput, ParsedOutput, RepairAction, SanitizationRule};

/// Port for extracting structured data from LLM output.
pub trait OutputParser: Send + Sync {
    /// Parses raw LLM output and extracts structured content.
    fn parse(&self, input: &LLMOutput) -> ParsedOutput;

    /// Checks if the content appears to be in the expected format.
    fn validate_format(&self, content: &str) -> bool;
}

/// Port for cleaning/sanitizing content.
pub trait OutputSanitizer: Send + Sync {
    /// Applies sanitization rules to content.
    fn sanitize(&self, content: &str) -> SanitizeResult;

    /// Returns the rules this sanitizer applies.
    fn rules(&self) -> &[SanitizationRule];

    /// Adds a custom rule.
    fn add_rule(&mut self, rule: SanitizationRule);
}

/// Result of sanitization.
#[derive(Debug, Clone)]
pub struct SanitizeResult {
    /// Sanitized content
    pub content: String,
    /// Rules that were applied
    pub applied_rules: Vec<String>,
    /// Repairs performed
    pub repairs: Vec<RepairAction>,
}

impl SanitizeResult {
    /// Creates a result with no changes.
    pub fn unchanged(content: String) -> Self {
        Self {
            content,
            applied_rules: Vec::new(),
            repairs: Vec::new(),
        }
    }

    /// Creates a sanitized result.
    pub fn sanitized(content: String, rules: Vec<String>, repairs: Vec<RepairAction>) -> Self {
        Self {
            content,
            applied_rules: rules,
            repairs,
        }
    }

    /// Returns true if content was modified.
    pub fn was_modified(&self) -> bool {
        !self.applied_rules.is_empty()
    }
}

/// Port for the complete self-healing pipeline.
pub trait SelfHealer: Send + Sync {
    /// Processes LLM output through the full healing pipeline.
    fn heal(&self, input: LLMOutput) -> ParsedOutput;

    /// Attempts to extract JSON from content.
    fn extract_json(&self, content: &str) -> Option<String>;

    /// Attempts to extract code from content.
    fn extract_code(&self, content: &str, language: Option<&str>) -> Option<String>;

    /// Reports if healing was successful.
    fn can_heal(&self, content: &str) -> HealabilityReport;
}

/// Report on whether content can be healed.
#[derive(Debug, Clone)]
pub struct HealabilityReport {
    /// Whether healing is possible
    pub healable: bool,
    /// Confidence in healing success
    pub confidence: f64,
    /// Issues detected
    pub issues: Vec<String>,
    /// Suggested strategies
    pub strategies: Vec<HealStrategy>,
}

/// Suggested healing strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealStrategy {
    /// Direct parsing (content looks valid)
    DirectParse,
    /// Extract from code blocks
    CodeBlockExtraction,
    /// JSON-specific repairs
    JSONRepair,
    /// Pattern-based heuristic fixes
    HeuristicRepair,
    /// Use another LLM to fix (last resort)
    LLMAssisted,
    /// Cannot be healed
    Unfixable,
}

impl HealabilityReport {
    /// Creates a report for healable content.
    pub fn healable(confidence: f64, strategies: Vec<HealStrategy>) -> Self {
        Self {
            healable: true,
            confidence,
            issues: Vec::new(),
            strategies,
        }
    }

    /// Creates a report for unhealable content.
    pub fn unhealable(issues: Vec<String>) -> Self {
        Self {
            healable: false,
            confidence: 0.0,
            issues,
            strategies: vec![HealStrategy::Unfixable],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_result_unchanged() {
        let result = SanitizeResult::unchanged("test".to_string());
        assert!(!result.was_modified());
    }

    #[test]
    fn test_sanitize_result_modified() {
        let result = SanitizeResult::sanitized(
            "fixed".to_string(),
            vec!["rule1".to_string()],
            Vec::new(),
        );
        assert!(result.was_modified());
    }

    #[test]
    fn test_healability_report() {
        let report = HealabilityReport::healable(
            0.9,
            vec![HealStrategy::JSONRepair, HealStrategy::HeuristicRepair],
        );
        assert!(report.healable);
        assert_eq!(report.strategies.len(), 2);

        let unhealable = HealabilityReport::unhealable(vec!["completely broken".to_string()]);
        assert!(!unhealable.healable);
    }
}
