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

pub mod channels;
pub mod compaction;
pub mod decision;
pub mod framework;
pub mod mcp;
pub mod parser;
pub mod persistence;
pub mod resilience;
pub mod security;
pub mod telemetry;

/// Convenient re-exports for common use cases
pub mod prelude {
    pub use crate::framework::{Agent, AgentHandle, EventBus, Hive, MemoryFragment, MemoryStore};
    pub use crate::telemetry::init_telemetry;
    // Updated MCP exports
    pub use crate::decision::{
        Decision, DecisionContext, DecisionEngine, GeminiProvider, LLMProvider, MinimaxProvider,
    };
    pub use crate::mcp::{
        EmptyContext, McpRegistry, McpServer, Prompt, Resource, Tool, ToolContext, ToolRegistry,
    };
    pub use crate::persistence::{
        DatabaseAdapter, DatabaseConfig, DatabaseHealth, DatabaseManager, Entity, EntityId, Filter,
        GraphAdapter, Pagination, PrimaryDbConfig, QueryResult, Sort, TypedDatabaseOps,
        VectorAdapter, VectorDbConfig,
    };
    // Resilience exports
    pub use crate::resilience::{
        CooldownReason, FailoverError, FailoverStrategy, InMemoryCooldownStore, ProviderHealth,
        ProviderId, ProviderRegistry, ResilientProvider, StochasticRotator,
    };
    // Compaction exports
    pub use crate::compaction::{
        CompactionConfig, CompactionResult, ContextOverflowRisk, LLMSummarizer, Message,
        MessageChunk, MessageRole, SessionContext, SimpleTokenEstimator, SummarizationStrategy,
        TokenCounter,
    };
    // Security exports
    pub use crate::security::{
        JSONValidator, OutputValidator, PIIRedactor, PIIType, RedactionConfig, RedactionResult,
        RegexPIIRedactor, SensitivityLevel, StructuredJSONValidator, ToolResultGuard,
        ValidationResult,
    };
    // Parser exports (Self-Healing)
    pub use crate::parser::{
        JsonExtractor, LLMOutput, MarkdownCleaner, OutputFormat, OutputParser, ParsedOutput,
        RepairAction, RepairSeverity, SelfHealer, SelfHealingPipeline,
    };
    // Channels exports (Multichannel)
    pub use crate::channels::{
        Attachment, Channel, ChannelAdapter, ChannelConfig, ChannelError, ChannelFeature,
        ChannelManager, ChannelMessage, ChannelStatus, CompositeLimiter, ContentRouter,
        DeliveryStatus, MessageContent, MessageFormatter, MessageId, MessageLogEntry, RateLimiter,
        ReceiveResult, RoutingStrategy, SendResult, SlackAdapter, SlackFormatter,
        SlidingWindowLimiter, ThreadContext, TokenBucketLimiter, WebSocketAdapter,
        WebSocketFormatter,
    };
    pub use anyhow::Result;
    pub use async_trait::async_trait;
}

// Re-export key types at crate root
pub use framework::{Agent, AgentHandle, Hive};
pub use mcp::{McpRegistry, McpServer, Tool};
pub use persistence::{DatabaseConfig, DatabaseManager};
