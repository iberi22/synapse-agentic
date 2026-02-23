//! Ports Layer: Abstract interfaces for security operations.

use crate::security::domain::{
    RedactionConfig, RedactionResult, ValidationError, ValidationResult,
};
use async_trait::async_trait;
// use std::future::Future; // Removed unused import

/// Port for PII detection and redaction.
#[async_trait]
pub trait PIIRedactor: Send + Sync {
    /// Scans text and redacts any detected PII.
    async fn redact(&self, text: &str, config: &RedactionConfig) -> RedactionResult;

    /// Checks if text contains any PII without redacting.
    async fn contains_pii(&self, text: &str) -> bool;
}

/// Port for validating tool output.
pub trait OutputValidator: Send + Sync {
    /// Validates content and returns result with any corrections.
    fn validate(&self, content: &str) -> ValidationResult;

    /// Attempts to repair malformed content.
    fn try_repair(&self, content: &str) -> Result<String, ValidationError>;
}

/// Port for JSON-specific validation.
pub trait JSONValidator: OutputValidator {
    /// Validates JSON structure only.
    fn validate_structure(&self, json: &str) -> Result<(), ValidationError>;

    /// Extracts valid JSON from mixed content.
    fn extract_json(&self, content: &str) -> Option<String>;

    /// Validates JSON against expected fields.
    fn validate_fields(&self, json: &str, required: &[&str]) -> Result<(), Vec<ValidationError>>;
}

/// Combined security guard for tool results.
#[async_trait]
pub trait ToolResultGuard: Send + Sync {
    /// Processes tool result through all security checks.
    async fn process(&self, result: ToolResult) -> GuardedResult;

    /// Sets the redaction configuration.
    fn set_redaction_config(&mut self, config: RedactionConfig);

    /// Enables or disables JSON validation.
    fn set_json_validation(&mut self, enabled: bool);
}

/// Input to the security guard.
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// The tool that produced this result
    pub tool_name: String,
    /// The raw output content
    pub content: String,
    /// Expected content type (json, text, etc.)
    pub content_type: ContentType,
    /// Whether this result will be sent to user
    pub user_facing: bool,
}

/// Type of content in tool result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    /// Plain text
    Text,
    /// JSON data
    JSON,
    /// Markdown content
    Markdown,
    /// Code snippet
    Code,
    /// Binary/unknown
    Binary,
}

/// Result after passing through security guard.
#[derive(Debug, Clone)]
pub struct GuardedResult {
    /// The processed (safe) content
    pub content: String,
    /// Original content hash for audit
    pub original_hash: String,
    /// Whether content was modified
    pub modified: bool,
    /// Summary of modifications
    pub modifications: Vec<Modification>,
    /// Whether result should be blocked
    pub blocked: bool,
    /// Reason for blocking (if blocked)
    pub block_reason: Option<String>,
}

/// A modification made by the guard.
#[derive(Debug, Clone)]
pub enum Modification {
    /// PII was redacted
    PIIRedacted {
        /// Number of PII instances found.
        count: usize,
        /// Types of PII that were redacted.
        types: Vec<String>,
    },
    /// JSON was repaired
    JSONRepaired {
        /// Description of the issue that was repaired.
        issue: String,
    },
    /// Content was truncated
    Truncated {
        /// Original content length in bytes.
        original_len: usize,
        /// New content length after truncation.
        new_len: usize,
    },
    /// Blocked patterns removed
    PatternsRemoved {
        /// Number of patterns that were removed.
        count: usize,
    },
}

impl GuardedResult {
    /// Creates a clean pass-through result.
    pub fn clean(content: String) -> Self {
        use sha2::{Digest, Sha256};
        let hash = format!("{:x}", Sha256::digest(content.as_bytes()));
        Self {
            content,
            original_hash: hash,
            modified: false,
            modifications: Vec::new(),
            blocked: false,
            block_reason: None,
        }
    }

    /// Creates a blocked result.
    pub fn blocked(reason: String) -> Self {
        Self {
            content: String::new(),
            original_hash: String::new(),
            modified: true,
            modifications: Vec::new(),
            blocked: true,
            block_reason: Some(reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guarded_result_clean() {
        let result = GuardedResult::clean("test content".to_string());
        assert!(!result.modified);
        assert!(!result.blocked);
        assert!(!result.original_hash.is_empty());
    }

    #[test]
    fn test_guarded_result_blocked() {
        let result = GuardedResult::blocked("critical PII detected".to_string());
        assert!(result.blocked);
        assert_eq!(
            result.block_reason,
            Some("critical PII detected".to_string())
        );
    }
}
