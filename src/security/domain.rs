//! Domain Layer: Security entities and value objects.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of Personally Identifiable Information (PII).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PIIType {
    /// Email addresses
    Email,
    /// Phone numbers
    Phone,
    /// Credit card numbers
    CreditCard,
    /// Social Security Numbers
    SSN,
    /// IP addresses
    IPAddress,
    /// API keys and tokens
    APIKey,
    /// Passwords (in plaintext)
    Password,
    /// AWS access keys
    AWSKey,
    /// Private keys (RSA, etc.)
    PrivateKey,
    /// Database connection strings
    ConnectionString,
    /// Generic secret patterns
    GenericSecret,
}

impl PIIType {
    /// Returns a human-readable name for this PII type.
    pub fn display_name(&self) -> &'static str {
        match self {
            PIIType::Email => "email address",
            PIIType::Phone => "phone number",
            PIIType::CreditCard => "credit card",
            PIIType::SSN => "SSN",
            PIIType::IPAddress => "IP address",
            PIIType::APIKey => "API key",
            PIIType::Password => "password",
            PIIType::AWSKey => "AWS key",
            PIIType::PrivateKey => "private key",
            PIIType::ConnectionString => "connection string",
            PIIType::GenericSecret => "secret",
        }
    }

    /// Returns the default redaction placeholder for this type.
    pub fn default_placeholder(&self) -> &'static str {
        match self {
            PIIType::Email => "[EMAIL_REDACTED]",
            PIIType::Phone => "[PHONE_REDACTED]",
            PIIType::CreditCard => "[CARD_REDACTED]",
            PIIType::SSN => "[SSN_REDACTED]",
            PIIType::IPAddress => "[IP_REDACTED]",
            PIIType::APIKey => "[API_KEY_REDACTED]",
            PIIType::Password => "[PASSWORD_REDACTED]",
            PIIType::AWSKey => "[AWS_KEY_REDACTED]",
            PIIType::PrivateKey => "[PRIVATE_KEY_REDACTED]",
            PIIType::ConnectionString => "[CONN_STRING_REDACTED]",
            PIIType::GenericSecret => "[SECRET_REDACTED]",
        }
    }
}

/// Sensitivity level for content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SensitivityLevel {
    /// No sensitive content detected
    None,
    /// Low sensitivity (e.g., public emails)
    Low,
    /// Medium sensitivity (e.g., internal IPs)
    Medium,
    /// High sensitivity (e.g., API keys)
    High,
    /// Critical sensitivity (e.g., private keys, passwords)
    Critical,
}

impl PIIType {
    /// Returns the sensitivity level for this PII type.
    pub fn sensitivity(&self) -> SensitivityLevel {
        match self {
            PIIType::Email => SensitivityLevel::Low,
            PIIType::Phone => SensitivityLevel::Medium,
            PIIType::IPAddress => SensitivityLevel::Low,
            PIIType::CreditCard => SensitivityLevel::High,
            PIIType::SSN => SensitivityLevel::Critical,
            PIIType::APIKey => SensitivityLevel::High,
            PIIType::Password => SensitivityLevel::Critical,
            PIIType::AWSKey => SensitivityLevel::Critical,
            PIIType::PrivateKey => SensitivityLevel::Critical,
            PIIType::ConnectionString => SensitivityLevel::High,
            PIIType::GenericSecret => SensitivityLevel::High,
        }
    }
}

/// Configuration for redaction behavior.
#[derive(Debug, Clone)]
pub struct RedactionConfig {
    /// PII types to detect and redact
    pub enabled_types: Vec<PIIType>,
    /// Minimum sensitivity level to redact
    pub min_sensitivity: SensitivityLevel,
    /// Custom placeholders for each type
    pub custom_placeholders: HashMap<PIIType, String>,
    /// Whether to log redactions
    pub log_redactions: bool,
    /// Whether to block content with critical PII instead of redacting
    pub block_critical: bool,
}

impl Default for RedactionConfig {
    fn default() -> Self {
        Self {
            enabled_types: vec![
                PIIType::Email,
                PIIType::Phone,
                PIIType::CreditCard,
                PIIType::SSN,
                PIIType::IPAddress,
                PIIType::APIKey,
                PIIType::Password,
                PIIType::AWSKey,
                PIIType::PrivateKey,
                PIIType::ConnectionString,
            ],
            min_sensitivity: SensitivityLevel::Low,
            custom_placeholders: HashMap::new(),
            log_redactions: true,
            block_critical: false,
        }
    }
}

impl RedactionConfig {
    /// Creates a strict config that blocks critical PII.
    pub fn strict() -> Self {
        Self {
            block_critical: true,
            min_sensitivity: SensitivityLevel::Medium,
            ..Default::default()
        }
    }

    /// Creates a permissive config (only redacts critical).
    pub fn permissive() -> Self {
        Self {
            min_sensitivity: SensitivityLevel::Critical,
            ..Default::default()
        }
    }

    /// Gets the placeholder for a PII type.
    pub fn placeholder(&self, pii_type: PIIType) -> &str {
        self.custom_placeholders
            .get(&pii_type)
            .map(|s| s.as_str())
            .unwrap_or_else(|| pii_type.default_placeholder())
    }
}

/// A single redaction that was performed.
#[derive(Debug, Clone)]
pub struct Redaction {
    /// Type of PII that was redacted
    pub pii_type: PIIType,
    /// Start position in original text
    pub start: usize,
    /// End position in original text
    pub end: usize,
    /// The placeholder used
    pub placeholder: String,
}

/// Result of a redaction operation.
#[derive(Debug, Clone)]
pub struct RedactionResult {
    /// The redacted text
    pub text: String,
    /// List of redactions performed
    pub redactions: Vec<Redaction>,
    /// Highest sensitivity level found
    pub max_sensitivity: SensitivityLevel,
    /// Whether content was blocked
    pub blocked: bool,
}

impl RedactionResult {
    /// Creates a result with no redactions.
    pub fn clean(text: String) -> Self {
        Self {
            text,
            redactions: Vec::new(),
            max_sensitivity: SensitivityLevel::None,
            blocked: false,
        }
    }

    /// Creates a blocked result.
    pub fn blocked(reason: &str) -> Self {
        Self {
            text: format!("[BLOCKED: {}]", reason),
            redactions: Vec::new(),
            max_sensitivity: SensitivityLevel::Critical,
            blocked: true,
        }
    }

    /// Returns true if any redactions were made.
    pub fn was_redacted(&self) -> bool {
        !self.redactions.is_empty()
    }

    /// Returns count of redactions by type.
    pub fn redaction_counts(&self) -> HashMap<PIIType, usize> {
        let mut counts = HashMap::new();
        for r in &self.redactions {
            *counts.entry(r.pii_type).or_insert(0) += 1;
        }
        counts
    }
}

/// Errors that can occur during validation.
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// JSON is malformed
    MalformedJSON { message: String, position: Option<usize> },
    /// Content exceeds size limits
    SizeExceeded { actual: usize, limit: usize },
    /// Content contains blocked patterns
    BlockedContent { reason: String },
    /// Schema validation failed
    SchemaViolation { path: String, message: String },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::MalformedJSON { message, position } => {
                match position {
                    Some(pos) => write!(f, "malformed JSON at position {}: {}", pos, message),
                    None => write!(f, "malformed JSON: {}", message),
                }
            }
            ValidationError::SizeExceeded { actual, limit } => {
                write!(f, "content size {} exceeds limit {}", actual, limit)
            }
            ValidationError::BlockedContent { reason } => {
                write!(f, "blocked content: {}", reason)
            }
            ValidationError::SchemaViolation { path, message } => {
                write!(f, "schema violation at '{}': {}", path, message)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Result of a validation operation.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,
    /// Errors found during validation
    pub errors: Vec<ValidationError>,
    /// The (possibly corrected) content
    pub content: String,
    /// Whether content was auto-corrected
    pub corrected: bool,
}

impl ValidationResult {
    /// Creates a successful validation result.
    pub fn ok(content: String) -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            content,
            corrected: false,
        }
    }

    /// Creates a successful result with corrections.
    pub fn corrected(content: String) -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            content,
            corrected: true,
        }
    }

    /// Creates a failed validation result.
    pub fn failed(errors: Vec<ValidationError>) -> Self {
        Self {
            valid: false,
            errors,
            content: String::new(),
            corrected: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pii_sensitivity() {
        assert_eq!(PIIType::Email.sensitivity(), SensitivityLevel::Low);
        assert_eq!(PIIType::Password.sensitivity(), SensitivityLevel::Critical);
        assert_eq!(PIIType::CreditCard.sensitivity(), SensitivityLevel::High);
    }

    #[test]
    fn test_redaction_config_placeholder() {
        let mut config = RedactionConfig::default();
        assert_eq!(config.placeholder(PIIType::Email), "[EMAIL_REDACTED]");

        config.custom_placeholders.insert(PIIType::Email, "***".to_string());
        assert_eq!(config.placeholder(PIIType::Email), "***");
    }

    #[test]
    fn test_redaction_result_counts() {
        let result = RedactionResult {
            text: "redacted".to_string(),
            redactions: vec![
                Redaction { pii_type: PIIType::Email, start: 0, end: 10, placeholder: "X".into() },
                Redaction { pii_type: PIIType::Email, start: 20, end: 30, placeholder: "X".into() },
                Redaction { pii_type: PIIType::Phone, start: 40, end: 50, placeholder: "Y".into() },
            ],
            max_sensitivity: SensitivityLevel::Low,
            blocked: false,
        };

        let counts = result.redaction_counts();
        assert_eq!(counts.get(&PIIType::Email), Some(&2));
        assert_eq!(counts.get(&PIIType::Phone), Some(&1));
    }
}
