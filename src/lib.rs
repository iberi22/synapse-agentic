//! # Synapse Agentic
//!
//! A modern Rust framework for building AI-native agentic systems with MCP support.
//!
//! ## Overview
//!
//! This crate provides:
//! - **Agent Framework**: Actor-based agents with typed messages and supervision
//! - **MCP Server**: Model Context Protocol for AI assistant integration
//! - **Multi-LLM Support**: OpenRouter, DeepSeek, Gemini, Grok with consensus
//! - **Tool System**: JSON-Schema validated capabilities
//! - **Persistence**: Multi-database abstraction (SurrealDB, PostgreSQL, pgvector)
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use synapse_agentic::prelude::*;
//!
//! #[derive(Debug)]
//! enum MyMessage {
//!     Ping,
//! }
//!
//! struct PingAgent;
//!
//! #[async_trait::async_trait]
//! impl Agent for PingAgent {
//!     type Input = MyMessage;
//!     fn name(&self) -> &str { "Ping" }
//!     async fn handle(&mut self, _msg: Self::Input) -> anyhow::Result<()> {
//!         println!("Pong!");
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Feature Flags
//!
//! - `mcp` - MCP Server support (default)
//! - `llm-providers` - All LLM integrations (default)
//! - `db-surreal` - SurrealDB support (default)
//! - `db-postgres` - PostgreSQL support
//! - `db-pgvector` - pgvector for embeddings
//! - `mcp-http` - HTTP transport for MCP

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod framework;
pub mod mcp;
pub mod decision;
pub mod persistence;
pub mod telemetry;
pub mod resilience;
pub mod compaction;
pub mod security;
pub mod parser;
pub mod channels;

/// Convenient re-exports for common use cases
pub mod prelude {
    pub use crate::telemetry::init_telemetry;
    pub use crate::framework::{Agent, AgentHandle, Hive, EventBus, MemoryStore, MemoryFragment};
    // Updated MCP exports
    pub use crate::mcp::{
        Tool, ToolRegistry, ToolContext, EmptyContext,
        McpServer, McpRegistry, Resource, Prompt
    };
    pub use crate::decision::{LLMProvider, DecisionContext, DecisionEngine, Decision};
    pub use crate::persistence::{
        DatabaseAdapter, TypedDatabaseOps, GraphAdapter, VectorAdapter,
        DatabaseManager, DatabaseHealth, DatabaseConfig, PrimaryDbConfig, VectorDbConfig,
        Entity, EntityId, QueryResult, Filter, Sort, Pagination
    };
    // Resilience exports
    pub use crate::resilience::{
        ProviderId, ProviderHealth, CooldownReason, FailoverStrategy, FailoverError,
        ProviderRegistry, ResilientProvider, StochasticRotator, InMemoryCooldownStore,
    };
    // Compaction exports
    pub use crate::compaction::{
        Message, MessageRole, MessageChunk, SessionContext,
        CompactionResult, CompactionConfig, ContextOverflowRisk,
        SummarizationStrategy, TokenCounter, SimpleTokenEstimator, LLMSummarizer,
    };
    // Security exports
    pub use crate::security::{
        PIIType, SensitivityLevel, RedactionConfig, RedactionResult, ValidationResult,
        PIIRedactor, OutputValidator, JSONValidator, ToolResultGuard,
        RegexPIIRedactor, StructuredJSONValidator,
    };
    // Parser exports (Self-Healing)
    pub use crate::parser::{
        LLMOutput, ParsedOutput, OutputFormat, RepairAction, RepairSeverity,
        OutputParser, SelfHealer, JsonExtractor, MarkdownCleaner, SelfHealingPipeline,
    };
    // Channels exports (Multichannel)
    pub use crate::channels::{
        Channel, MessageId, MessageContent, ChannelMessage, ThreadContext,
        Attachment, ChannelConfig, ChannelStatus, DeliveryStatus,
        ChannelError, SendResult, ReceiveResult, ChannelAdapter, MessageFormatter,
        ChannelFeature, RateLimiter, SlackAdapter, SlackFormatter, WebSocketAdapter,
        WebSocketFormatter, TokenBucketLimiter, SlidingWindowLimiter, CompositeLimiter,
        ChannelManager, RoutingStrategy, MessageLogEntry, ContentRouter,
    };
    pub use async_trait::async_trait;
    pub use anyhow::Result;
}

// Re-export key types at crate root
pub use framework::{Agent, AgentHandle, Hive};
pub use persistence::{DatabaseManager, DatabaseConfig};
pub use mcp::{Tool, McpServer, McpRegistry};

