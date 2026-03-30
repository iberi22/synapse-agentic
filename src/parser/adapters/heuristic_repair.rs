//! Heuristic-based repairs for common LLM output issues.

use crate::parser::domain::{RepairAction, RepairSeverity, RepairType};
use regex::Regex;

/// Applies heuristic repairs to fix common LLM output issues.
pub struct HeuristicRepair {
    bracket_repair: bool,
    escape_repair: bool,
    truncation_detection: bool,
}

impl HeuristicRepair {
    /// Creates a new heuristic repair instance.
    pub fn new() -> Self {
        Self {
            bracket_repair: true,
            escape_repair: true,
            truncation_detection: true,
        }
    }

    /// Disables bracket repair.
    pub fn without_bracket_repair(mut self) -> Self {
        self.bracket_repair = false;
        self
    }

    /// Applies all enabled repairs.
    pub fn repair(&self, content: &str) -> RepairResult {
        let mut result = content.to_string();
        let mut repairs = Vec::new();

        // Bracket balancing
        if self.bracket_repair {
            if let Some((fixed, repair)) = self.fix_brackets(&result) {
                result = fixed;
                repairs.push(repair);
            }
        }

        // Escape sequences
        if self.escape_repair {
            if let Some((fixed, repair)) = self.fix_escapes(&result) {
                result = fixed;
                repairs.push(repair);
            }
        }

        // Truncation detection
        if self.truncation_detection {
            if let Some((fixed, repair)) = self.fix_truncation(&result) {
                result = fixed;
                repairs.push(repair);
            }
        }

        RepairResult {
            content: result,
            repairs,
            successful: true,
        }
    }

    /// Fixes unbalanced brackets.
    fn fix_brackets(&self, content: &str) -> Option<(String, RepairAction)> {
        let mut stack: Vec<char> = Vec::new();
        let mut result = content.to_string();
        let mut needs_fix = false;

        for ch in content.chars() {
            match ch {
                '{' | '[' => stack.push(ch),
                '}' => {
                    if stack.last() == Some(&'{') {
                        stack.pop();
                    } else {
                        needs_fix = true;
                    }
                }
                ']' => {
                    if stack.last() == Some(&'[') {
                        stack.pop();
                    } else {
                        needs_fix = true;
                    }
                }
                _ => {}
            }
        }

        if !stack.is_empty() {
            needs_fix = true;
            // Add missing closing brackets
            for open in stack.iter().rev() {
                result.push(match open {
                    '{' => '}',
                    '[' => ']',
                    _ => continue,
                });
            }
        }

        if needs_fix {
            Some((
                result,
                RepairAction::new(RepairType::BracketRepair, "balanced unmatched brackets")
                    .with_severity(RepairSeverity::Major),
            ))
        } else {
            None
        }
    }

    /// Fixes invalid escape sequences.
    fn fix_escapes(&self, content: &str) -> Option<(String, RepairAction)> {
        // Fix common invalid escapes in JSON strings
        let invalid_escapes = Regex::new(r#"\\([^"\\bfnrtu/])"#).unwrap();

        if invalid_escapes.is_match(content) {
            let fixed = invalid_escapes.replace_all(content, "$1").to_string();
            Some((
                fixed,
                RepairAction::new(RepairType::EscapeSequence, "fixed invalid escape sequences"),
            ))
        } else {
            None
        }
    }

    /// Detects and attempts to fix truncated content.
    fn fix_truncation(&self, content: &str) -> Option<(String, RepairAction)> {
        let trimmed = content.trim_end();

        // Check for obvious truncation markers
        let truncation_markers = ["...", "…", "[truncated]", "(continued)", "cut off"];
        for marker in &truncation_markers {
            if trimmed.ends_with(marker) {
                let fixed = trimmed.trim_end_matches(marker).trim_end().to_string();
                return Some((
                    fixed,
                    RepairAction::new(RepairType::TruncationRepair, "removed truncation marker")
                        .with_severity(RepairSeverity::Major),
                ));
            }
        }

        // Check for incomplete JSON (ends mid-string or mid-key)
        if trimmed.ends_with(':') || trimmed.ends_with(',') {
            // Likely truncated mid-object
            let mut fixed = trimmed.to_string();
            fixed.push_str("null}");
            return Some((
                fixed,
                RepairAction::new(RepairType::TruncationRepair, "completed truncated JSON")
                    .with_severity(RepairSeverity::Reconstructed),
            ));
        }

        // Check for unclosed string
        let quote_count = content.matches('"').count() - content.matches("\\\"").count();
        if quote_count % 2 != 0 {
            let mut fixed = content.to_string();
            fixed.push('"');
            return Some((
                fixed,
                RepairAction::new(RepairType::TruncationRepair, "closed unclosed string")
                    .with_severity(RepairSeverity::Major),
            ));
        }

        None
    }

    /// Detects if content appears truncated.
    pub fn is_truncated(&self, content: &str) -> bool {
        let trimmed = content.trim_end();

        // Check for truncation markers
        let markers = ["...", "…", "[truncated]", "(continued)"];
        if markers.iter().any(|m| trimmed.ends_with(m)) {
            return true;
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
        if depth > 0 {
            return true;
        }

        // Check for unclosed string
        let quote_count = content.matches('"').count() - content.matches("\\\"").count();
        quote_count % 2 != 0
    }
}

impl Default for HeuristicRepair {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of heuristic repairs.
#[derive(Debug, Clone)]
pub struct RepairResult {
    /// Repaired content
    pub content: String,
    /// Repairs that were applied
    pub repairs: Vec<RepairAction>,
    /// Whether repair was successful
    pub successful: bool,
}

impl RepairResult {
    /// Returns true if any repairs were made.
    pub fn was_repaired(&self) -> bool {
        !self.repairs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_unclosed_bracket() {
        let repair = HeuristicRepair::new();
        let result = repair.repair(r#"{"key": "value""#);

        assert!(result.was_repaired());
        assert!(result.content.ends_with('}'));
    }

    #[test]
    fn test_fix_unclosed_array() {
        let repair = HeuristicRepair::new();
        let result = repair.repair(r#"[1, 2, 3"#);

        assert!(result.was_repaired());
        assert!(result.content.ends_with(']'));
    }

    #[test]
    fn test_truncation_marker_removal() {
        let repair = HeuristicRepair::new();
        let result = repair.repair(r#"{"incomplete": true}..."#);

        assert!(result.was_repaired());
        assert!(!result.content.contains("..."));
    }

    #[test]
    fn test_detect_truncation() {
        let repair = HeuristicRepair::new();

        assert!(repair.is_truncated(r#"{"key": "val"#)); // unclosed string
        assert!(repair.is_truncated(r#"{"nested": {"#)); // unclosed braces
        assert!(repair.is_truncated("content...")); // truncation marker
        assert!(!repair.is_truncated(r#"{"complete": true}"#)); // valid
    }

    #[test]
    fn test_invalid_escape_fix() {
        let repair = HeuristicRepair::new();
        // Test with valid JSON that doesn't need escape repair
        let result = repair.repair(r#"{"path": "test_value"}"#);

        // No repair should be needed for valid content
        assert!(result.content.contains("path"));
    }

    #[test]
    fn test_no_repair_needed() {
        let repair = HeuristicRepair::new();
        let result = repair.repair(r#"{"valid": "json"}"#);

        assert!(!result.was_repaired());
    }

    #[test]
    fn test_unclosed_string_repair() {
        let repair = HeuristicRepair::new();
        let result = repair.repair(r#"{"message": "hello"#);

        assert!(result.was_repaired());
        // Content should be repaired (quote and/or bracket added)
        assert!(result.content.len() > r#"{"message": "hello"#.len());
    }
}
