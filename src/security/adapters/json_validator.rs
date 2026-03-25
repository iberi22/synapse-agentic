//! JSON validation adapter for tool outputs.

use crate::security::domain::{ValidationError, ValidationResult};
use crate::security::ports::{JSONValidator, OutputValidator};
use regex::Regex;

/// Structured JSON validator with repair capabilities.
pub struct StructuredJSONValidator {
    max_size: usize,
    max_depth: usize,
    json_extractor: Regex,
}

impl StructuredJSONValidator {
    /// Creates a new validator with default limits.
    pub fn new() -> Self {
        Self {
            max_size: 1024 * 1024, // 1MB default
            max_depth: 50,
            json_extractor: Regex::new(
                r"(?s)(\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\}|\[[^\[\]]*(?:\[[^\[\]]*\][^\[\]]*)*\])",
            )
            .expect("invalid JSON extractor regex"),
        }
    }

    /// Sets maximum allowed content size.
    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_size = size;
        self
    }

    /// Sets maximum allowed nesting depth.
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Attempts to fix common JSON issues.
    fn attempt_repairs(&self, content: &str) -> Option<String> {
        let mut result = content.to_string();

        // Fix 1: Remove trailing commas before } or ]
        let trailing_comma = Regex::new(r",(\s*[}\]])").unwrap();
        result = trailing_comma.replace_all(&result, "$1").to_string();

        // Fix 2: Add missing quotes to unquoted keys
        let unquoted_keys = Regex::new(r"(\{|,)\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*:").unwrap();
        result = unquoted_keys.replace_all(&result, r#"$1"$2":"#).to_string();

        // Fix 3: Replace single quotes with double quotes
        result = result.replace('\'', "\"");

        // Fix 4: Handle None/null equivalents
        result = result.replace("None", "null");
        result = result.replace("True", "true");
        result = result.replace("False", "false");

        // Validate the repairs worked
        if serde_json::from_str::<serde_json::Value>(&result).is_ok() {
            Some(result)
        } else {
            None
        }
    }

    /// Calculates JSON nesting depth.
    fn calculate_depth(&self, content: &str) -> usize {
        let mut max_depth: usize = 0;
        let mut current_depth: usize = 0;

        for ch in content.chars() {
            match ch {
                '{' | '[' => {
                    current_depth += 1;
                    max_depth = max_depth.max(current_depth);
                }
                '}' | ']' => {
                    current_depth = current_depth.saturating_sub(1);
                }
                _ => {}
            }
        }

        max_depth
    }
}

impl Default for StructuredJSONValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputValidator for StructuredJSONValidator {
    fn validate(&self, content: &str) -> ValidationResult {
        // Check size limit
        if content.len() > self.max_size {
            return ValidationResult::failed(vec![ValidationError::SizeExceeded {
                actual: content.len(),
                limit: self.max_size,
            }]);
        }

        // Check depth
        let depth = self.calculate_depth(content);
        if depth > self.max_depth {
            return ValidationResult::failed(vec![ValidationError::SchemaViolation {
                path: "/".to_string(),
                message: format!("nesting depth {} exceeds limit {}", depth, self.max_depth),
            }]);
        }

        // Try to parse as JSON
        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(_) => ValidationResult::ok(content.to_string()),
            Err(e) => {
                // Try repairs
                if let Some(repaired) = self.attempt_repairs(content) {
                    ValidationResult::corrected(repaired)
                } else {
                    let position = if e.is_syntax() {
                        Some(e.column())
                    } else {
                        None
                    };
                    ValidationResult::failed(vec![ValidationError::MalformedJSON {
                        message: e.to_string(),
                        position,
                    }])
                }
            }
        }
    }

    fn try_repair(&self, content: &str) -> Result<String, ValidationError> {
        self.attempt_repairs(content)
            .ok_or_else(|| ValidationError::MalformedJSON {
                message: "unable to repair JSON".to_string(),
                position: None,
            })
    }
}

impl JSONValidator for StructuredJSONValidator {
    fn validate_structure(&self, json: &str) -> Result<(), ValidationError> {
        serde_json::from_str::<serde_json::Value>(json)
            .map(|_| ())
            .map_err(|e| ValidationError::MalformedJSON {
                message: e.to_string(),
                position: if e.is_syntax() {
                    Some(e.column())
                } else {
                    None
                },
            })
    }

    fn extract_json(&self, content: &str) -> Option<String> {
        // First try: parse whole content
        if serde_json::from_str::<serde_json::Value>(content).is_ok() {
            return Some(content.to_string());
        }

        // Second try: find JSON in markdown code blocks
        let code_block = Regex::new(r"```(?:json)?\s*([\s\S]*?)```").unwrap();
        if let Some(caps) = code_block.captures(content) {
            let extracted = caps.get(1).map(|m| m.as_str().trim().to_string());
            if let Some(ref json) = extracted {
                if serde_json::from_str::<serde_json::Value>(json).is_ok() {
                    return extracted;
                }
            }
        }

        // Third try: regex extraction
        for cap in self.json_extractor.find_iter(content) {
            let candidate = cap.as_str();
            if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                return Some(candidate.to_string());
            }
        }

        None
    }

    fn validate_fields(&self, json: &str, required: &[&str]) -> Result<(), Vec<ValidationError>> {
        let value: serde_json::Value = serde_json::from_str(json).map_err(|e| {
            vec![ValidationError::MalformedJSON {
                message: e.to_string(),
                position: None,
            }]
        })?;

        let obj = value.as_object().ok_or_else(|| {
            vec![ValidationError::SchemaViolation {
                path: "/".to_string(),
                message: "expected object at root".to_string(),
            }]
        })?;

        let missing: Vec<ValidationError> = required
            .iter()
            .filter(|field| !obj.contains_key(**field))
            .map(|field| ValidationError::SchemaViolation {
                path: format!("/{}", field),
                message: "required field missing".to_string(),
            })
            .collect();

        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_json() {
        let validator = StructuredJSONValidator::new();
        let result = validator.validate(r#"{"key": "value"}"#);

        assert!(result.valid);
        assert!(!result.corrected);
    }

    #[test]
    fn test_trailing_comma_repair() {
        let validator = StructuredJSONValidator::new();
        let result = validator.validate(r#"{"key": "value",}"#);

        assert!(result.valid);
        assert!(result.corrected);
        assert_eq!(result.content, r#"{"key": "value"}"#);
    }

    #[test]
    fn test_single_quote_repair() {
        let validator = StructuredJSONValidator::new();
        let result = validator.validate(r#"{'key': 'value'}"#);

        assert!(result.valid);
        assert!(result.corrected);
    }

    #[test]
    fn test_python_literals_repair() {
        let validator = StructuredJSONValidator::new();
        let result = validator.validate(r#"{"enabled": True, "data": None}"#);

        assert!(result.valid);
        assert!(result.corrected);
        assert!(result.content.contains("true"));
        assert!(result.content.contains("null"));
    }

    #[test]
    fn test_size_exceeded() {
        let validator = StructuredJSONValidator::new().with_max_size(10);
        let result = validator.validate(r#"{"key": "this is a long value"}"#);

        assert!(!result.valid);
        assert!(matches!(
            result.errors[0],
            ValidationError::SizeExceeded { .. }
        ));
    }

    #[test]
    fn test_extract_json_from_markdown() {
        let validator = StructuredJSONValidator::new();
        let content = r#"Here is the response:
```json
{"status": "ok"}
```
Done!"#;

        let extracted = validator.extract_json(content);
        assert_eq!(extracted, Some(r#"{"status": "ok"}"#.to_string()));
    }

    #[test]
    fn test_extract_json_plain() {
        let validator = StructuredJSONValidator::new();
        let content = r#"Result: {"value": 42} end"#;

        let extracted = validator.extract_json(content);
        assert_eq!(extracted, Some(r#"{"value": 42}"#.to_string()));
    }

    #[test]
    fn test_validate_required_fields() {
        let validator = StructuredJSONValidator::new();

        let result = validator.validate_fields(r#"{"name": "test", "id": 1}"#, &["name", "id"]);
        assert!(result.is_ok());

        let result = validator.validate_fields(r#"{"name": "test"}"#, &["name", "id"]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().len(), 1);
    }

    #[test]
    fn test_depth_calculation() {
        let validator = StructuredJSONValidator::new().with_max_depth(3);

        // Depth 2 - should pass
        let result = validator.validate(r#"{"a": {"b": 1}}"#);
        assert!(result.valid);

        // Depth 4 - should fail
        let result = validator.validate(r#"{"a": {"b": {"c": {"d": 1}}}}"#);
        assert!(!result.valid);
    }

    #[test]
    fn test_invalid_json() {
        let validator = StructuredJSONValidator::new();
        let result = validator.validate(r#"{"key": broken}"#);

        assert!(!result.valid);
        assert!(matches!(
            result.errors[0],
            ValidationError::MalformedJSON { .. }
        ));
    }
}
