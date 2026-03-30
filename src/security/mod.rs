//! # Security Module
//!
//! Provides tool result validation, PII redaction, and output sanitization.
//!
//! ## Architecture (Hexagonal)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    APPLICATION LAYER                        │
//! │  ┌─────────────────┐  ┌─────────────────────────────────┐  │
//! │  │   ToolGuard     │  │ SanitizationPolicy (Port)       │  │
//! │  │   (Use Case)    │  │                                 │  │
//! │  └─────────────────┘  └─────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      DOMAIN LAYER                           │
//! │  ┌─────────────┐ ┌─────────────┐ ┌───────────────────────┐  │
//! │  │ PIIType     │ │ Redaction   │ │  ValidationResult     │  │
//! │  └─────────────┘ └─────────────┘ └───────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  INFRASTRUCTURE LAYER                       │
//! │  ┌─────────────────────┐  ┌───────────────────────────────┐ │
//! │  │ RegexPIIRedactor    │  │ JSONValidator                 │ │
//! │  └─────────────────────┘  └───────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//! ```

// Domain: Pure business logic
pub mod domain;

// Application: Ports and use cases
pub mod ports;

// Infrastructure: Concrete implementations
pub mod adapters;

// Re-exports
pub use adapters::{RegexPIIRedactor, StructuredJSONValidator};
pub use domain::{
    PIIType, Redaction, RedactionConfig, RedactionResult, SensitivityLevel, ValidationError,
    ValidationResult,
};
pub use ports::{
    ContentType, GuardedResult, JSONValidator, Modification, OutputValidator, PIIRedactor,
    ToolResult, ToolResultGuard,
};
