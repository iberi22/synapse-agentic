//! # MCP - Model Context Protocol
//!
//! Implementation of the Model Context Protocol for AI assistant integration.
//!
//! ## Components
//!
//! - [`Tool`] - Trait for defining callable capabilities
//! - [`ToolRegistry`] - Container for registering and managing tools
//! - [`ToolContext`] - Context passed to tools during execution

pub mod http_transport;
mod prompt;
mod registry;
mod resource;
mod server;
mod tool;

pub use http_transport::HttpTransport;
pub use prompt::{GetPromptResult, Prompt, PromptArgument, PromptMessage, PromptMessageContent};
pub use registry::{McpRegistry, ToolRegistry};
pub use resource::Resource;
pub use server::{McpServer, McpServerConfig};
pub use tool::{BoxToolContext, EmptyContext, Tool, ToolContext, ToolContextExt};
