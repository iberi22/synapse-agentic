//! Tool trait for MCP capabilities.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::any::Any;

/// Context passed to tools during execution.
///
/// This allows tools to access application state without being tightly coupled.
/// Implement this trait to provide your application's context to tools.
///
/// # Example
///
/// ```rust,no_run
/// use synapse_agentic::mcp::ToolContext;
/// use std::any::Any;
///
/// struct MyAppContext {
///     db_pool: String, // Simulated DB pool
///     user_id: String,
/// }
///
/// impl ToolContext for MyAppContext {
///     fn get(&self, key: &str) -> Option<&dyn Any> {
///         match key {
///             "user_id" => Some(&self.user_id as &dyn Any),
///             _ => None,
///         }
///     }
/// }
/// ```
pub trait ToolContext: Send + Sync {
    /// Gets a value from the context by key.
    fn get(&self, key: &str) -> Option<&dyn Any>;
}

/// Extension trait for typed context access.
pub trait ToolContextExt: ToolContext {
    /// Gets a typed value from the context.
    fn get_typed<T: 'static>(&self, key: &str) -> Option<&T> {
        self.get(key).and_then(|v| v.downcast_ref::<T>())
    }
}

impl<C: ToolContext + ?Sized> ToolContextExt for C {}

/// A boxed ToolContext for dynamic dispatch.
pub type BoxToolContext = Box<dyn ToolContext>;

/// Empty context for tools that don't need application state.
pub struct EmptyContext;

impl ToolContext for EmptyContext {
    fn get(&self, _key: &str) -> Option<&dyn Any> {
        None
    }
}

/// The `Tool` trait defines a capability that can be invoked by an agent or MCP client.
///
/// Tools are the primary way to expose functionality to AI assistants.
/// Each tool has a name, description, JSON Schema for parameters, and an async call method.
///
/// # Example
///
/// ```rust,no_run
/// use synapse_agentic::mcp::{Tool, ToolContext};
/// use async_trait::async_trait;
/// use serde::{Deserialize, Serialize};
/// use schemars::JsonSchema;
///
/// #[derive(Deserialize, JsonSchema)]
/// struct AddArgs {
///     a: i32,
///     b: i32,
/// }
///
/// struct AddTool;
///
/// #[async_trait]
/// impl Tool for AddTool {
///     fn name(&self) -> &str { "add" }
///     fn description(&self) -> &str { "Adds two numbers" }
///
///     fn parameters(&self) -> serde_json::Value {
///         serde_json::json!({
///             "type": "object",
///             "properties": {
///                 "a": { "type": "integer" },
///                 "b": { "type": "integer" }
///             },
///             "required": ["a", "b"]
///         })
///     }
///
///     async fn call(&self, _ctx: &dyn ToolContext, args: serde_json::Value) -> anyhow::Result<serde_json::Value> {
///         let args: AddArgs = serde_json::from_value(args)?;
///         Ok(serde_json::json!({ "result": args.a + args.b }))
///     }
/// }
/// ```
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the unique name of the tool.
    fn name(&self) -> &str;

    /// Returns a human-readable description of what the tool does.
    fn description(&self) -> &str;

    /// Returns the JSON Schema for the tool's parameters.
    fn parameters(&self) -> Value;

    /// Executes the tool with the given context and arguments.
    async fn call(&self, ctx: &dyn ToolContext, args: Value) -> Result<Value>;
}
