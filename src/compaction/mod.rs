//! # Compaction Module
//!
//! Provides intelligent context window management through hierarchical
//! summarization of conversation history.
//!
//! ## Architecture (Hexagonal)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    APPLICATION LAYER                        │
//! │  ┌─────────────────┐  ┌─────────────────────────────────┐  │
//! │  │ CompactContext  │  │ SummarizationStrategy (Port)    │  │
//! │  │   (Use Case)    │  │                                 │  │
//! │  └─────────────────┘  └─────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      DOMAIN LAYER                           │
//! │  ┌─────────────┐ ┌─────────────┐ ┌───────────────────────┐  │
//! │  │MessageChunk │ │SessionCtx   │ │  CompactionResult     │  │
//! │  └─────────────┘ └─────────────┘ └───────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  INFRASTRUCTURE LAYER                       │
//! │  ┌─────────────────────┐  ┌───────────────────────────────┐ │
//! │  │ TokenEstimator      │  │ LLMSummarizer                 │ │
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
    Message, MessageRole, MessageChunk, SessionContext,
    CompactionResult, CompactionConfig, ContextOverflowRisk,
};
pub use ports::{SummarizationStrategy, TokenCounter};
pub use adapters::{SimpleTokenEstimator, LLMSummarizer};
