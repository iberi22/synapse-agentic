//! # MCP - Model Context Protocol
//!
//! Implementation of the Model Context Protocol for AI assistant integration.
//!
//! ## Components
//!
//! - [`Tool`] - Trait for defining callable capabilities
//! - [`ToolRegistry`] - Container for registering and managing tools
//! - [`ToolContext`] - Context passed to tools during execution

mod tool;
mod resource;
mod prompt;
mod registry;
mod server;

pub use tool::{Tool, ToolContext, ToolContextExt, BoxToolContext, EmptyContext};
pub use resource::Resource;
pub use prompt::{Prompt, PromptArgument, GetPromptResult, PromptMessage, PromptMessageContent};
pub use registry::{ToolRegistry, McpRegistry};
pub use server::{McpServer, McpServerConfig};
