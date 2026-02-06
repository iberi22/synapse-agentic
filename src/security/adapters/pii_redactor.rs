//! Regex-based PII redactor adapter.

use crate::security::domain::{
    PIIType, Redaction, RedactionConfig, RedactionResult, SensitivityLevel,
};
use crate::security::ports::PIIRedactor;
use async_trait::async_trait;
use regex::Regex;
use std::collections::HashMap;

/// Regex-based implementation of PII detection and redaction.
pub struct RegexPIIRedactor {
    patterns: HashMap<PIIType, Regex>,
}

impl RegexPIIRedactor {
    /// Creates a new redactor with default patterns.
    pub fn new() -> Self {
        let mut patterns = HashMap::new();

        // Email: simple pattern for common emails
        patterns.insert(
            PIIType::Email,
            Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}")
                .expect("invalid email regex"),
        );

        // Phone: various formats (US-centric + international)
        patterns.insert(
            PIIType::Phone,
            Regex::new(r"(?:\+?1[-.\s]?)?\(?[0-9]{3}\)?[-.\s]?[0-9]{3}[-.\s]?[0-9]{4}")
                .expect("invalid phone regex"),
        );

        // Credit Card: major card patterns (Visa, MC, Amex, Discover)
        patterns.insert(
            PIIType::CreditCard,
            Regex::new(r"\b(?:4[0-9]{12}(?:[0-9]{3})?|5[1-5][0-9]{14}|3[47][0-9]{13}|6(?:011|5[0-9]{2})[0-9]{12})\b")
                .expect("invalid credit card regex"),
        );

        // SSN: XXX-XX-XXXX format
        patterns.insert(
            PIIType::SSN,
            Regex::new(r"\b[0-9]{3}-[0-9]{2}-[0-9]{4}\b")
                .expect("invalid SSN regex"),
        );

        // IP Address: IPv4
        patterns.insert(
            PIIType::IPAddress,
            Regex::new(r"\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b")
                .expect("invalid IP regex"),
        );

        // API Key patterns (generic long alphanumeric)
        patterns.insert(
            PIIType::APIKey,
            Regex::new(r#"(?i)(?:api[_-]?key|apikey|api_secret)["']?\s*[:=]\s*["']?([a-zA-Z0-9_-]{20,})["']?"#)
                .expect("invalid API key regex"),
        );

        // Password patterns (in config/env files)
        patterns.insert(
            PIIType::Password,
            Regex::new(r#"(?i)(?:password|passwd|pwd)["']?\s*[:=]\s*["']?([^\s"']{4,})["']?"#)
                .expect("invalid password regex"),
        );

        // AWS Access Key
        patterns.insert(
            PIIType::AWSKey,
            Regex::new(r"(?:AKIA|ABIA|ACCA|ASIA)[A-Z0-9]{16}")
                .expect("invalid AWS key regex"),
        );

        // Private Key markers
        patterns.insert(
            PIIType::PrivateKey,
            Regex::new(r"-----BEGIN\s+(?:RSA\s+)?PRIVATE\s+KEY-----")
                .expect("invalid private key regex"),
        );

        // Connection strings
        patterns.insert(
            PIIType::ConnectionString,
            Regex::new(r"(?i)(?:mongodb|postgres|mysql|redis|amqp)://[^\s]+")
                .expect("invalid connection string regex"),
        );

        // Generic secrets (bearer tokens, etc.)
        patterns.insert(
            PIIType::GenericSecret,
            Regex::new(r#"(?i)(?:bearer|token|secret|auth)["']?\s*[:=]\s*["']?([a-zA-Z0-9_.-]{20,})["']?"#)
                .expect("invalid generic secret regex"),
        );

        Self { patterns }
    }

    /// Adds a custom pattern for a PII type.
    pub fn with_pattern(mut self, pii_type: PIIType, pattern: &str) -> Result<Self, regex::Error> {
        self.patterns.insert(pii_type, Regex::new(pattern)?);
        Ok(self)
    }

    /// Finds all PII matches in text.
    fn find_matches(&self, text: &str, config: &RedactionConfig) -> Vec<(PIIType, usize, usize)> {
        let mut matches = Vec::new();

        for pii_type in &config.enabled_types {
            if pii_type.sensitivity() < config.min_sensitivity {
                continue;
            }

            if let Some(pattern) = self.patterns.get(pii_type) {
                for m in pattern.find_iter(text) {
                    matches.push((*pii_type, m.start(), m.end()));
                }
            }
        }

        // Sort by position (reverse for safe replacement)
        matches.sort_by(|a, b| b.1.cmp(&a.1));
        matches
    }
}

impl Default for RegexPIIRedactor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PIIRedactor for RegexPIIRedactor {
    async fn redact(&self, text: &str, config: &RedactionConfig) -> RedactionResult {
        let matches = self.find_matches(text, config);

        if matches.is_empty() {
            return RedactionResult::clean(text.to_string());
        }

        // Check for critical PII if blocking is enabled
        let max_sensitivity = matches
            .iter()
            .map(|(t, _, _)| t.sensitivity())
            .max()
            .unwrap_or(SensitivityLevel::None);

        if config.block_critical && max_sensitivity == SensitivityLevel::Critical {
            return RedactionResult::blocked("critical PII detected");
        }

        // Perform redactions (from end to start to preserve positions)
        let mut result = text.to_string();
        let mut redactions = Vec::new();

        for (pii_type, start, end) in matches {
            let placeholder = config.placeholder(pii_type).to_string();
            result.replace_range(start..end, &placeholder);
            redactions.push(Redaction {
                pii_type,
                start,
                end,
                placeholder,
            });
        }

        // Reverse redactions to get original order
        redactions.reverse();

        RedactionResult {
            text: result,
            redactions,
            max_sensitivity,
            blocked: false,
        }
    }

    async fn contains_pii(&self, text: &str) -> bool {
        for pattern in self.patterns.values() {
            if pattern.is_match(text) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_email_redaction() {
        let redactor = RegexPIIRedactor::new();
        let config = RedactionConfig::default();

        let result = redactor
            .redact("Contact me at john.doe@example.com please", &config)
            .await;

        assert!(result.was_redacted());
        assert!(result.text.contains("[EMAIL_REDACTED]"));
        assert!(!result.text.contains("john.doe@example.com"));
    }

    #[tokio::test]
    async fn test_phone_redaction() {
        let redactor = RegexPIIRedactor::new();
        let config = RedactionConfig::default();

        let result = redactor
            .redact("Call me at (555) 123-4567", &config)
            .await;

        assert!(result.was_redacted());
        assert!(result.text.contains("[PHONE_REDACTED]"));
    }

    #[tokio::test]
    async fn test_ssn_redaction() {
        let redactor = RegexPIIRedactor::new();
        let config = RedactionConfig::default();

        let result = redactor
            .redact("SSN: 123-45-6789", &config)
            .await;

        assert!(result.was_redacted());
        assert!(result.text.contains("[SSN_REDACTED]"));
        assert_eq!(result.max_sensitivity, SensitivityLevel::Critical);
    }

    #[tokio::test]
    async fn test_block_critical() {
        let redactor = RegexPIIRedactor::new();
        let config = RedactionConfig::strict();

        let result = redactor
            .redact("Password: mysecretpass123", &config)
            .await;

        assert!(result.blocked);
    }

    #[tokio::test]
    async fn test_multiple_pii_types() {
        let redactor = RegexPIIRedactor::new();
        let config = RedactionConfig::default();

        let result = redactor
            .redact("Email: test@test.com, Phone: 555-123-4567", &config)
            .await;

        assert_eq!(result.redactions.len(), 2);
        let counts = result.redaction_counts();
        assert_eq!(counts.get(&PIIType::Email), Some(&1));
        assert_eq!(counts.get(&PIIType::Phone), Some(&1));
    }

    #[tokio::test]
    async fn test_aws_key_detection() {
        let redactor = RegexPIIRedactor::new();
        let config = RedactionConfig::default();

        let result = redactor
            .redact("AWS_ACCESS_KEY=AKIAIOSFODNN7EXAMPLE", &config)
            .await;

        assert!(result.was_redacted());
        assert!(result.text.contains("[AWS_KEY_REDACTED]"));
    }

    #[tokio::test]
    async fn test_ip_address_redaction() {
        let redactor = RegexPIIRedactor::new();
        let config = RedactionConfig::default();

        let result = redactor
            .redact("Server IP: 192.168.1.100", &config)
            .await;

        assert!(result.was_redacted());
        assert!(result.text.contains("[IP_REDACTED]"));
    }

    #[tokio::test]
    async fn test_clean_content() {
        let redactor = RegexPIIRedactor::new();
        let config = RedactionConfig::default();

        let result = redactor
            .redact("This is clean content with no PII", &config)
            .await;

        assert!(!result.was_redacted());
        assert!(!result.blocked);
    }

    #[tokio::test]
    async fn test_contains_pii() {
        let redactor = RegexPIIRedactor::new();

        assert!(redactor.contains_pii("test@example.com").await);
        assert!(!redactor.contains_pii("no pii here").await);
    }
}
