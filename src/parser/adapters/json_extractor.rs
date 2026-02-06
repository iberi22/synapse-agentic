//! JSON extraction from mixed LLM output.

use crate::parser::domain::{LLMOutput, OutputFormat, ParsedOutput, RepairAction, RepairType};
use crate::parser::ports::OutputParser;
use regex::Regex;

/// Extracts JSON from mixed content (code blocks, prose, etc.).
pub struct JsonExtractor {
    code_block_pattern: Regex,
    json_object_pattern: Regex,
    json_array_pattern: Regex,
}

impl JsonExtractor {
    /// Creates a new JSON extractor.
    pub fn new() -> Self {
        Self {
            // Matches ```json ... ``` or ``` ... ```
            code_block_pattern: Regex::new(r"(?s)```(?:json)?\s*\n?(.*?)\n?```")
                .expect("invalid code block regex"),
            // Matches JSON objects { ... }
            json_object_pattern: Regex::new(r"(?s)\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\}")
                .expect("invalid json object regex"),
            // Matches JSON arrays [ ... ]
            json_array_pattern: Regex::new(r"(?s)\[[^\[\]]*(?:\[[^\[\]]*\][^\[\]]*)*\]")
                .expect("invalid json array regex"),
        }
    }

    /// Attempts to extract JSON from content.
    pub fn extract(&self, content: &str) -> Option<(String, Vec<RepairAction>)> {
        let mut repairs = Vec::new();

        // Strategy 1: Try direct parse
        if serde_json::from_str::<serde_json::Value>(content).is_ok() {
            return Some((content.to_string(), repairs));
        }

        // Strategy 2: Extract from code blocks
        if let Some(caps) = self.code_block_pattern.captures(content) {
            if let Some(inner) = caps.get(1) {
                let extracted = inner.as_str().trim();
                if let Some(repaired) = self.try_repair_json(extracted) {
                    repairs.push(RepairAction::new(
                        RepairType::CodeBlockExtraction,
                        "extracted JSON from code block",
                    ));
                    return Some((repaired.0, [repairs, repaired.1].concat()));
                }
            }
        }

        // Strategy 3: Find JSON object in content
        for cap in self.json_object_pattern.find_iter(content) {
            let candidate = cap.as_str();
            if let Some(repaired) = self.try_repair_json(candidate) {
                repairs.push(RepairAction::new(
                    RepairType::JSONRepair,
                    "extracted JSON object from mixed content",
                ));
                return Some((repaired.0, [repairs, repaired.1].concat()));
            }
        }

        // Strategy 4: Find JSON array in content
        for cap in self.json_array_pattern.find_iter(content) {
            let candidate = cap.as_str();
            if let Some(repaired) = self.try_repair_json(candidate) {
                repairs.push(RepairAction::new(
                    RepairType::JSONRepair,
                    "extracted JSON array from mixed content",
                ));
                return Some((repaired.0, [repairs, repaired.1].concat()));
            }
        }

        None
    }

    /// Attempts to repair common JSON issues.
    fn try_repair_json(&self, content: &str) -> Option<(String, Vec<RepairAction>)> {
        let mut result = content.to_string();
        let mut repairs = Vec::new();

        // Direct parse first
        if serde_json::from_str::<serde_json::Value>(&result).is_ok() {
            return Some((result, repairs));
        }

        // Fix 1: Trailing commas
        let trailing = Regex::new(r",(\s*[}\]])").unwrap();
        let new_result = trailing.replace_all(&result, "$1").to_string();
        if new_result != result {
            repairs.push(RepairAction::new(
                RepairType::TrailingComma,
                "removed trailing comma(s)",
            ));
            result = new_result;
        }

        // Fix 2: Single quotes to double
        if result.contains('\'') && !result.contains('"') {
            result = result.replace('\'', "\"");
            repairs.push(RepairAction::new(
                RepairType::QuoteNormalization,
                "converted single quotes to double",
            ));
        }

        // Fix 3: Python literals
        let before = result.clone();
        result = result.replace("None", "null")
            .replace("True", "true")
            .replace("False", "false");
        if result != before {
            repairs.push(RepairAction::new(
                RepairType::PythonLiteralConversion,
                "converted Python literals to JSON",
            ));
        }

        // Fix 4: Unquoted keys
        let unquoted = Regex::new(r"(\{|,)\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*:").unwrap();
        let new_result = unquoted.replace_all(&result, r#"$1"$2":"#).to_string();
        if new_result != result {
            repairs.push(RepairAction::new(
                RepairType::JSONRepair,
                "quoted unquoted keys",
            ));
            result = new_result;
        }

        // Fix 5: Control characters
        let control_chars = Regex::new(r"[\x00-\x1f]").unwrap();
        let new_result = control_chars.replace_all(&result, "").to_string();
        if new_result != result {
            repairs.push(RepairAction::new(
                RepairType::ControlCharRemoval,
                "removed control characters",
            ));
            result = new_result;
        }

        // Validate result
        if serde_json::from_str::<serde_json::Value>(&result).is_ok() {
            Some((result, repairs))
        } else {
            None
        }
    }
}

impl Default for JsonExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputParser for JsonExtractor {
    fn parse(&self, input: &LLMOutput) -> ParsedOutput {
        match self.extract(&input.content) {
            Some((content, repairs)) => ParsedOutput::repaired(content, OutputFormat::JSON, repairs),
            None => ParsedOutput::clean(input.content.clone(), OutputFormat::Unknown),
        }
    }

    fn validate_format(&self, content: &str) -> bool {
        serde_json::from_str::<serde_json::Value>(content).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_json() {
        let extractor = JsonExtractor::new();
        let result = extractor.extract(r#"{"key": "value"}"#);

        assert!(result.is_some());
        let (content, repairs) = result.unwrap();
        assert_eq!(content, r#"{"key": "value"}"#);
        assert!(repairs.is_empty());
    }

    #[test]
    fn test_json_in_code_block() {
        let extractor = JsonExtractor::new();
        let input = r#"Here's the result:
```json
{"status": "ok", "count": 42}
```
That's all!"#;

        let result = extractor.extract(input);
        assert!(result.is_some());
        let (content, repairs) = result.unwrap();
        assert!(content.contains("status"));
        assert!(repairs.iter().any(|r| r.repair_type == RepairType::CodeBlockExtraction));
    }

    #[test]
    fn test_trailing_comma_repair() {
        let extractor = JsonExtractor::new();
        let result = extractor.extract(r#"{"key": "value",}"#);

        assert!(result.is_some());
        let (content, repairs) = result.unwrap();
        assert_eq!(content, r#"{"key": "value"}"#);
        assert!(repairs.iter().any(|r| r.repair_type == RepairType::TrailingComma));
    }

    #[test]
    fn test_python_literals_repair() {
        let extractor = JsonExtractor::new();
        let result = extractor.extract(r#"{"enabled": True, "data": None}"#);

        assert!(result.is_some());
        let (content, repairs) = result.unwrap();
        assert!(content.contains("true"));
        assert!(content.contains("null"));
        assert!(repairs.iter().any(|r| r.repair_type == RepairType::PythonLiteralConversion));
    }

    #[test]
    fn test_mixed_content_extraction() {
        let extractor = JsonExtractor::new();
        let input = r#"Based on the analysis, here are the results:

{"findings": ["issue1", "issue2"], "severity": "high"}

Please review the above findings."#;

        let result = extractor.extract(input);
        assert!(result.is_some());
        let (content, _) = result.unwrap();
        assert!(content.contains("findings"));
    }

    #[test]
    fn test_invalid_json() {
        let extractor = JsonExtractor::new();
        let result = extractor.extract("this is not json at all");
        assert!(result.is_none());
    }
}
