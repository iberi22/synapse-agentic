//! Domain Layer: Parser entities and value objects.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Format of the expected output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Plain text
    Text,
    /// JSON data
    JSON,
    /// XML data
    XML,
    /// YAML data
    YAML,
    /// Code in specific language
    Code(CodeLanguage),
    /// Markdown content
    Markdown,
    /// Unknown format
    Unknown,
}

/// Code language for code extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodeLanguage {
    /// Rust programming language
    Rust,
    /// Python programming language
    Python,
    /// JavaScript programming language
    JavaScript,
    /// TypeScript programming language
    TypeScript,
    /// SQL queries
    SQL,
    /// Shell/Bash scripts
    Shell,
    /// Other unspecified language
    Other,
}

impl CodeLanguage {
    /// Parse from string identifier.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Self::Rust,
            "python" | "py" => Self::Python,
            "javascript" | "js" => Self::JavaScript,
            "typescript" | "ts" => Self::TypeScript,
            "sql" => Self::SQL,
            "bash" | "sh" | "shell" | "zsh" => Self::Shell,
            _ => Self::Other,
        }
    }
}

/// Raw LLM output before processing.
#[derive(Debug, Clone)]
pub struct LLMOutput {
    /// Raw content from LLM
    pub content: String,
    /// Model that generated this
    pub model: Option<String>,
    /// Expected output format
    pub expected_format: OutputFormat,
    /// Time taken to generate
    pub generation_time: Option<Duration>,
    /// Token count if available
    pub token_count: Option<usize>,
}

impl LLMOutput {
    /// Creates a new LLM output.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            model: None,
            expected_format: OutputFormat::Unknown,
            generation_time: None,
            token_count: None,
        }
    }

    /// Sets the expected format.
    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.expected_format = format;
        self
    }

    /// Sets the model name.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// Severity of a repair action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RepairSeverity {
    /// Cosmetic fix (whitespace, formatting)
    Cosmetic,
    /// Minor fix (trailing commas, quotes)
    Minor,
    /// Moderate fix (structure repair)
    Moderate,
    /// Major fix (significant reconstruction)
    Major,
    /// Complete reconstruction
    Reconstructed,
}

/// A single repair action that was performed.
#[derive(Debug, Clone)]
pub struct RepairAction {
    /// Type of repair
    pub repair_type: RepairType,
    /// Severity of the repair
    pub severity: RepairSeverity,
    /// Description of what was fixed
    pub description: String,
    /// Position in original content (if applicable)
    pub position: Option<usize>,
    /// Original value (if captured)
    pub original: Option<String>,
    /// New value (if applicable)
    pub replacement: Option<String>,
}

/// Types of repairs that can be performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairType {
    /// Removed markdown code block markers
    CodeBlockExtraction,
    /// Fixed JSON syntax
    JSONRepair,
    /// Fixed trailing comma
    TrailingComma,
    /// Converted Python literals (True→true, None→null)
    PythonLiteralConversion,
    /// Fixed quote style
    QuoteNormalization,
    /// Removed control characters
    ControlCharRemoval,
    /// Fixed unescaped characters
    EscapeSequence,
    /// Removed thinking/reasoning text
    ThinkingRemoval,
    /// Fixed truncated content
    TruncationRepair,
    /// Normalized whitespace
    WhitespaceNormalization,
    /// Fixed bracket matching
    BracketRepair,
    /// Other repair
    Other,
}

impl RepairAction {
    /// Creates a new repair action.
    pub fn new(repair_type: RepairType, description: impl Into<String>) -> Self {
        Self {
            repair_type,
            severity: repair_type.default_severity(),
            description: description.into(),
            position: None,
            original: None,
            replacement: None,
        }
    }

    /// Sets the severity.
    pub fn with_severity(mut self, severity: RepairSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Sets position info.
    pub fn at_position(mut self, pos: usize) -> Self {
        self.position = Some(pos);
        self
    }

    /// Sets original value.
    pub fn with_original(mut self, original: impl Into<String>) -> Self {
        self.original = Some(original.into());
        self
    }
}

impl RepairType {
    /// Returns default severity for this repair type.
    pub fn default_severity(&self) -> RepairSeverity {
        match self {
            RepairType::WhitespaceNormalization => RepairSeverity::Cosmetic,
            RepairType::TrailingComma => RepairSeverity::Minor,
            RepairType::QuoteNormalization => RepairSeverity::Minor,
            RepairType::PythonLiteralConversion => RepairSeverity::Minor,
            RepairType::ControlCharRemoval => RepairSeverity::Minor,
            RepairType::EscapeSequence => RepairSeverity::Minor,
            RepairType::CodeBlockExtraction => RepairSeverity::Moderate,
            RepairType::JSONRepair => RepairSeverity::Moderate,
            RepairType::ThinkingRemoval => RepairSeverity::Moderate,
            RepairType::BracketRepair => RepairSeverity::Major,
            RepairType::TruncationRepair => RepairSeverity::Major,
            RepairType::Other => RepairSeverity::Moderate,
        }
    }
}

/// Processed and validated output.
#[derive(Debug, Clone)]
pub struct ParsedOutput {
    /// The cleaned/repaired content
    pub content: String,
    /// Detected format
    pub format: OutputFormat,
    /// Whether any repairs were needed
    pub was_repaired: bool,
    /// List of repairs performed
    pub repairs: Vec<RepairAction>,
    /// Confidence in the result (0.0-1.0)
    pub confidence: f64,
    /// Hash of original content
    pub original_hash: String,
}

impl ParsedOutput {
    /// Creates a clean output (no repairs needed).
    pub fn clean(content: String, format: OutputFormat) -> Self {
        use sha2::{Sha256, Digest};
        let hash = format!("{:x}", Sha256::digest(content.as_bytes()));
        Self {
            content,
            format,
            was_repaired: false,
            repairs: Vec::new(),
            confidence: 1.0,
            original_hash: hash,
        }
    }

    /// Creates a repaired output.
    pub fn repaired(content: String, format: OutputFormat, repairs: Vec<RepairAction>) -> Self {
        use sha2::{Sha256, Digest};
        let hash = format!("{:x}", Sha256::digest(content.as_bytes()));
        let confidence = Self::calculate_confidence(&repairs);
        Self {
            content,
            format,
            was_repaired: !repairs.is_empty(),
            repairs,
            confidence,
            original_hash: hash,
        }
    }

    /// Calculates confidence based on repairs performed.
    fn calculate_confidence(repairs: &[RepairAction]) -> f64 {
        if repairs.is_empty() {
            return 1.0;
        }

        let penalty: f64 = repairs.iter().map(|r| {
            match r.severity {
                RepairSeverity::Cosmetic => 0.01,
                RepairSeverity::Minor => 0.05,
                RepairSeverity::Moderate => 0.15,
                RepairSeverity::Major => 0.30,
                RepairSeverity::Reconstructed => 0.50,
            }
        }).sum();

        (1.0 - penalty).max(0.1)
    }

    /// Returns max severity of all repairs.
    pub fn max_severity(&self) -> Option<RepairSeverity> {
        self.repairs.iter().map(|r| r.severity).max()
    }
}

/// Configurable sanitization rule.
#[derive(Debug, Clone)]
pub struct SanitizationRule {
    /// Name of the rule
    pub name: String,
    /// Whether rule is enabled
    pub enabled: bool,
    /// Pattern to match (regex string)
    pub pattern: String,
    /// Replacement string
    pub replacement: String,
    /// Repair type for logging
    pub repair_type: RepairType,
}

impl SanitizationRule {
    /// Creates a new rule.
    pub fn new(name: impl Into<String>, pattern: impl Into<String>, replacement: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: true,
            pattern: pattern.into(),
            replacement: replacement.into(),
            repair_type: RepairType::Other,
        }
    }

    /// Sets the repair type.
    pub fn with_type(mut self, repair_type: RepairType) -> Self {
        self.repair_type = repair_type;
        self
    }

    /// Disables the rule.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_output_builder() {
        let output = LLMOutput::new("test content")
            .with_format(OutputFormat::JSON)
            .with_model("gpt-4");

        assert_eq!(output.content, "test content");
        assert_eq!(output.expected_format, OutputFormat::JSON);
        assert_eq!(output.model, Some("gpt-4".to_string()));
    }

    #[test]
    fn test_repair_severity_ordering() {
        assert!(RepairSeverity::Cosmetic < RepairSeverity::Minor);
        assert!(RepairSeverity::Minor < RepairSeverity::Major);
        assert!(RepairSeverity::Major < RepairSeverity::Reconstructed);
    }

    #[test]
    fn test_confidence_calculation() {
        let repairs_minor = vec![
            RepairAction::new(RepairType::TrailingComma, "fixed comma"),
        ];
        let output = ParsedOutput::repaired("{}".into(), OutputFormat::JSON, repairs_minor);
        assert!(output.confidence > 0.9);

        let repairs_major = vec![
            RepairAction::new(RepairType::BracketRepair, "fixed brackets")
                .with_severity(RepairSeverity::Major),
        ];
        let output = ParsedOutput::repaired("{}".into(), OutputFormat::JSON, repairs_major);
        assert!(output.confidence < 0.8);
    }

    #[test]
    fn test_code_language_parsing() {
        assert_eq!(CodeLanguage::from_str("rust"), CodeLanguage::Rust);
        assert_eq!(CodeLanguage::from_str("py"), CodeLanguage::Python);
        assert_eq!(CodeLanguage::from_str("JavaScript"), CodeLanguage::JavaScript);
        assert_eq!(CodeLanguage::from_str("unknown"), CodeLanguage::Other);
    }
}
