//! # Parser Module (Self-Healing LLM Output)
//!
//! Provides automatic repair and sanitization of malformed LLM responses.
//!
//! ## Architecture (Hexagonal)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     PARSER MODULE                           │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Domain Layer                                               │
//! │  ├── LLMOutput (raw response structure)                    │
//! │  ├── ParsedOutput (validated/repaired output)              │
//! │  ├── RepairAction (what was fixed)                         │
//! │  └── SanitizationRule (configurable cleanup rules)         │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Ports Layer                                                │
//! │  ├── OutputParser (extract structured data)                │
//! │  ├── OutputSanitizer (clean artifacts)                     │
//! │  └── SelfHealer (auto-repair pipeline)                     │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Adapters Layer                                             │
//! │  ├── JsonExtractor (extract JSON from mixed content)       │
//! │  ├── MarkdownCleaner (remove markdown artifacts)           │
//! │  └── HeuristicRepair (pattern-based fixes)                 │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use synapse_agentic::parser::{SelfHealingPipeline, JsonExtractor};
//!
//! let pipeline = SelfHealingPipeline::new()
//!     .with_json_extraction()
//!     .with_markdown_cleanup();
//!
//! let raw = r#"Here's the result: ```json\n{"status": "ok"}\n```"#;
//! let parsed = pipeline.process(raw)?;
//! assert_eq!(parsed.content, r#"{"status": "ok"}"#);
//! ```

pub mod adapters;
pub mod domain;
pub mod ports;

// Domain exports
pub use domain::{
    LLMOutput, OutputFormat, ParsedOutput, RepairAction, RepairSeverity, SanitizationRule,
};

// Port exports
pub use ports::{OutputParser, OutputSanitizer, SelfHealer};

// Adapter exports
pub use adapters::{HeuristicRepair, JsonExtractor, MarkdownCleaner, SelfHealingPipeline};
