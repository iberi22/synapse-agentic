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
pub use domain::{
    PIIType, RedactionConfig, RedactionResult, ValidationResult,
    ValidationError, SensitivityLevel, Redaction,
};
pub use ports::{
    PIIRedactor, OutputValidator, JSONValidator, ToolResultGuard,
    ToolResult, ContentType, GuardedResult, Modification,
};
pub use adapters::{RegexPIIRedactor, StructuredJSONValidator};
