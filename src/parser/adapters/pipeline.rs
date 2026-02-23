//! Self-healing pipeline that combines all repair strategies.

use crate::parser::adapters::{HeuristicRepair, JsonExtractor, MarkdownCleaner};
use crate::parser::domain::{LLMOutput, OutputFormat, ParsedOutput, RepairAction};
use crate::parser::ports::{HealStrategy, HealabilityReport, OutputSanitizer, SelfHealer};
use regex::Regex;

/// Complete self-healing pipeline for LLM output.
pub struct SelfHealingPipeline {
    json_extractor: JsonExtractor,
    markdown_cleaner: MarkdownCleaner,
    heuristic_repair: HeuristicRepair,
    enable_json: bool,
    enable_markdown: bool,
    enable_heuristics: bool,
}

impl SelfHealingPipeline {
    /// Creates a new pipeline with all features enabled.
    pub fn new() -> Self {
        Self {
            json_extractor: JsonExtractor::new(),
            markdown_cleaner: MarkdownCleaner::new(),
            heuristic_repair: HeuristicRepair::new(),
            enable_json: true,
            enable_markdown: true,
            enable_heuristics: true,
        }
    }

    /// Enables JSON extraction.
    pub fn with_json_extraction(mut self) -> Self {
        self.enable_json = true;
        self
    }

    /// Enables markdown cleanup.
    pub fn with_markdown_cleanup(mut self) -> Self {
        self.enable_markdown = true;
        self
    }

    /// Enables heuristic repairs.
    pub fn with_heuristics(mut self) -> Self {
        self.enable_heuristics = true;
        self
    }

    /// Disables JSON extraction.
    pub fn without_json(mut self) -> Self {
        self.enable_json = false;
        self
    }

    /// Disables markdown cleanup.
    pub fn without_markdown(mut self) -> Self {
        self.enable_markdown = false;
        self
    }

    /// Process content through the pipeline.
    fn process_content(
        &self,
        content: &str,
        expected: OutputFormat,
    ) -> (String, Vec<RepairAction>) {
        let mut result = content.to_string();
        let mut repairs = Vec::new();

        // Step 1: Markdown cleanup
        if self.enable_markdown {
            let sanitized = self.markdown_cleaner.sanitize(&result);
            if sanitized.was_modified() {
                result = sanitized.content;
                repairs.extend(sanitized.repairs);
            }
        }

        // Step 2: Format-specific extraction
        if self.enable_json && (expected == OutputFormat::JSON || expected == OutputFormat::Unknown)
        {
            if let Some((extracted, json_repairs)) = self.json_extractor.extract(&result) {
                result = extracted;
                repairs.extend(json_repairs);
            }
        }

        // Step 3: Heuristic repairs
        if self.enable_heuristics {
            let repair_result = self.heuristic_repair.repair(&result);
            if repair_result.was_repaired() {
                result = repair_result.content;
                repairs.extend(repair_result.repairs);
            }
        }

        (result, repairs)
    }
}

impl Default for SelfHealingPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfHealer for SelfHealingPipeline {
    fn heal(&self, input: LLMOutput) -> ParsedOutput {
        let (content, repairs) = self.process_content(&input.content, input.expected_format);

        // Determine final format
        let format = if serde_json::from_str::<serde_json::Value>(&content).is_ok() {
            OutputFormat::JSON
        } else {
            input.expected_format
        };

        if repairs.is_empty() {
            ParsedOutput::clean(content, format)
        } else {
            ParsedOutput::repaired(content, format, repairs)
        }
    }

    fn extract_json(&self, content: &str) -> Option<String> {
        self.json_extractor.extract(content).map(|(c, _)| c)
    }

    fn extract_code(&self, content: &str, language: Option<&str>) -> Option<String> {
        // Pattern for code blocks with optional language
        let pattern = if let Some(lang) = language {
            format!(r"(?s)```{}\s*\n?(.*?)\n?```", regex::escape(lang))
        } else {
            r"(?s)```(?:\w+)?\s*\n?(.*?)\n?```".to_string()
        };

        let regex = Regex::new(&pattern).ok()?;
        regex
            .captures(content)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().trim().to_string())
    }

    fn can_heal(&self, content: &str) -> HealabilityReport {
        let mut strategies = Vec::new();
        let mut issues = Vec::new();
        let mut confidence: f64 = 1.0;

        // Check if it's valid as-is
        if serde_json::from_str::<serde_json::Value>(content).is_ok() {
            return HealabilityReport::healable(1.0, vec![HealStrategy::DirectParse]);
        }

        // Check for code blocks
        if content.contains("```") {
            strategies.push(HealStrategy::CodeBlockExtraction);
            confidence = confidence.min(0.9);
        }

        // Check for common JSON issues
        if content.contains("True") || content.contains("False") || content.contains("None") {
            strategies.push(HealStrategy::JSONRepair);
            confidence = confidence.min(0.85);
        }

        // Check for truncation
        if self.heuristic_repair.is_truncated(content) {
            strategies.push(HealStrategy::HeuristicRepair);
            issues.push("content appears truncated".to_string());
            confidence = confidence.min(0.6);
        }

        // Check for unbalanced brackets
        let mut depth: i32 = 0;
        for ch in content.chars() {
            match ch {
                '{' | '[' => depth += 1,
                '}' | ']' => depth -= 1,
                _ => {}
            }
        }
        if depth != 0 {
            strategies.push(HealStrategy::HeuristicRepair);
            issues.push(format!("unbalanced brackets (depth: {})", depth));
            confidence = confidence.min(0.5);
        }

        if strategies.is_empty() {
            // No known strategies, might still contain JSON
            if content.contains('{') || content.contains('[') {
                strategies.push(HealStrategy::JSONRepair);
                confidence = 0.4;
            } else {
                return HealabilityReport::unhealable(vec![
                    "no JSON-like content found".to_string()
                ]);
            }
        }

        HealabilityReport {
            healable: true,
            confidence,
            issues,
            strategies,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_pipeline_clean_json() {
        let pipeline = SelfHealingPipeline::new();
        let input = LLMOutput::new(r#"{"status": "ok"}"#).with_format(OutputFormat::JSON);

        let result = pipeline.heal(input);
        assert!(!result.was_repaired);
        assert_eq!(result.format, OutputFormat::JSON);
    }

    #[test]
    fn test_pipeline_with_code_block() {
        let pipeline = SelfHealingPipeline::new();
        let input = LLMOutput::new(
            r#"Here's the result:
```json
{"value": 42}
```"#,
        )
        .with_format(OutputFormat::JSON);

        let result = pipeline.heal(input);
        assert!(result.was_repaired);
        assert!(result.content.contains("42"));
        assert!(!result.content.contains("```"));
    }

    #[test]
    fn test_pipeline_with_thinking() {
        let pipeline = SelfHealingPipeline::new();
        let input = LLMOutput::new(
            r#"<thinking>Let me think...</thinking>
{"answer": "hello"}"#,
        )
        .with_format(OutputFormat::JSON);

        let result = pipeline.heal(input);
        assert!(result.was_repaired);
        assert!(!result.content.contains("thinking"));
        assert!(result.content.contains("hello"));
    }

    #[test]
    fn test_pipeline_with_python_literals() {
        let pipeline = SelfHealingPipeline::new();
        let input =
            LLMOutput::new(r#"{"active": True, "count": None}"#).with_format(OutputFormat::JSON);

        let result = pipeline.heal(input);
        assert!(result.was_repaired);
        assert!(result.content.contains("true"));
        assert!(result.content.contains("null"));
    }

    #[test]
    fn test_extract_code() {
        let pipeline = SelfHealingPipeline::new();
        let content = r#"Here's the code:
```rust
fn main() {
    println!("Hello");
}
```"#;

        let code = pipeline.extract_code(content, Some("rust"));
        assert!(code.is_some());
        assert!(code.unwrap().contains("println"));
    }

    #[test]
    fn test_can_heal_valid() {
        let pipeline = SelfHealingPipeline::new();
        let report = pipeline.can_heal(r#"{"valid": true}"#);

        assert!(report.healable);
        assert_eq!(report.confidence, 1.0);
    }

    #[test]
    fn test_can_heal_truncated() {
        let pipeline = SelfHealingPipeline::new();
        let report = pipeline.can_heal(r#"{"incomplete": "#);

        assert!(report.healable);
        assert!(report.confidence < 1.0);
        assert!(!report.issues.is_empty());
    }

    #[test]
    fn test_can_heal_unfixable() {
        let pipeline = SelfHealingPipeline::new();
        let report = pipeline.can_heal("just plain text with no structure");

        assert!(!report.healable);
    }

    #[test]
    fn test_high_confidence_output() {
        let pipeline = SelfHealingPipeline::new();
        let input = LLMOutput::new(r#"{"simple": "json"}"#).with_format(OutputFormat::JSON);

        let result = pipeline.heal(input);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn test_reduced_confidence_after_repairs() {
        let pipeline = SelfHealingPipeline::new();
        let input = LLMOutput::new(r#"{"trailing": "comma",}"#).with_format(OutputFormat::JSON);

        let result = pipeline.heal(input);
        assert!(result.was_repaired);
        assert!(result.confidence < 1.0);
    }
}
