//! Registry for managing MCP components (Tools, Resources, Prompts).

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;
use anyhow::Result;

use super::tool::{Tool, ToolContext};
use super::resource::Resource;
use super::prompt::Prompt;

/// Information about a registered tool.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Information about a registered resource.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceInfo {
    pub uri: String,
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    pub description: Option<String>,
}

/// Information about a registered prompt.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PromptInfo {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Vec<crate::mcp::prompt::PromptArgument>,
}

/// Central registry for all MCP capabilities.
pub struct McpRegistry {
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
    resources: RwLock<HashMap<String, Arc<dyn Resource>>>,
    prompts: RwLock<HashMap<String, Arc<dyn Prompt>>>,
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl McpRegistry {
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
            resources: RwLock::new(HashMap::new()),
            prompts: RwLock::new(HashMap::new()),
        }
    }

    // --- Tools ---

    /// Registers a new tool in the registry.
    ///
    /// # Arguments
    ///
    /// * `tool` - The tool implementation to register.
    pub async fn register_tool(&self, tool: impl Tool + 'static) {
        let name = tool.name().to_string();
        self.tools.write().await.insert(name, Arc::new(tool));
    }

    /// Registers a boxed tool implementation.
    ///
    /// # Arguments
    ///
    /// * `tool` - The boxed tool to register.
    pub async fn register_tool_boxed(&self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.write().await.insert(name, Arc::from(tool));
    }

    /// Unregisters a tool by name.
    pub async fn unregister_tool(&self, name: &str) -> bool {
        self.tools.write().await.remove(name).is_some()
    }

    /// Lists all registered tools with their metadata.
    ///
    /// # Returns
    ///
    /// Vector of [`ToolInfo`] containing tool names, descriptions, and schemas.
    pub async fn list_tools(&self) -> Vec<ToolInfo> {
        self.tools
            .read()
            .await
            .values()
            .map(|t| ToolInfo {
                name: t.name().to_string(),
                description: t.description().to_string(),
                input_schema: t.parameters(),
            })
            .collect()
    }

    /// Calls a registered tool by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the tool to call.
    /// * `ctx` - The tool execution context.
    /// * `args` - Arguments to pass to the tool.
    ///
    /// # Returns
    ///
    /// The result of the tool execution.
    ///
    /// # Errors
    ///
    /// Returns an error if the tool is not found or execution fails.
    pub async fn call_tool(&self, name: &str, ctx: &dyn ToolContext, args: Value) -> Result<Value> {
        let tools = self.tools.read().await;
        let tool = tools
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", name))?
            .clone();
        drop(tools);
        tool.call(ctx, args).await
    }

    // --- Resources ---

    /// Registers a new resource in the registry.
    ///
    /// # Arguments
    ///
    /// * `resource` - The resource to register.
    pub async fn register_resource(&self, resource: impl Resource + 'static) {
        let uri = resource.uri().to_string();
        self.resources.write().await.insert(uri, Arc::new(resource));
    }

    /// Registers a boxed resource implementation.
    ///
    /// # Arguments
    ///
    /// * `resource` - The boxed resource to register.
    pub async fn register_resource_boxed(&self, resource: Box<dyn Resource>) {
        let uri = resource.uri().to_string();
        self.resources.write().await.insert(uri, Arc::from(resource));
    }

    /// Lists all registered resources.
    ///
    /// # Returns
    ///
    /// Vector of [`ResourceInfo`] containing resource metadata.
    pub async fn list_resources(&self) -> Vec<ResourceInfo> {
        self.resources
            .read()
            .await
            .values()
            .map(|r| ResourceInfo {
                uri: r.uri().to_string(),
                name: r.name().to_string(),
                mime_type: r.mime_type().map(String::from),
                description: r.description().map(String::from),
            })
            .collect()
    }

    /// Reads a resource by its URI.
    ///
    /// # Arguments
    ///
    /// * `uri` - The URI of the resource to read.
    ///
    /// # Returns
    ///
    /// The resource content as a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the resource is not found or read fails.
    pub async fn read_resource(&self, uri: &str) -> Result<String> {
        let resources = self.resources.read().await;
        let resource = resources
            .get(uri)
            .ok_or_else(|| anyhow::anyhow!("Resource not found: {}", uri))?
            .clone();
        drop(resources);
        resource.read().await
    }

    // --- Prompts ---

    /// Registers a new prompt template.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt template to register.
    pub async fn register_prompt(&self, prompt: impl Prompt + 'static) {
        let name = prompt.name().to_string();
        self.prompts.write().await.insert(name, Arc::new(prompt));
    }

    /// Registers a boxed prompt implementation.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The boxed prompt to register.
    pub async fn register_prompt_boxed(&self, prompt: Box<dyn Prompt>) {
        let name = prompt.name().to_string();
        self.prompts.write().await.insert(name, Arc::from(prompt));
    }

    pub async fn list_prompts(&self) -> Vec<PromptInfo> {
        self.prompts
            .read()
            .await
            .values()
            .map(|p| PromptInfo {
                name: p.name().to_string(),
                description: p.description().map(String::from),
                arguments: p.arguments(),
            })
            .collect()
    }

    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: HashMap<String, String>,
    ) -> Result<crate::mcp::prompt::GetPromptResult> {
        let prompts = self.prompts.read().await;
        let prompt = prompts
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Prompt not found: {}", name))?
            .clone();
        drop(prompts);

        let args_val = serde_json::to_value(arguments)?;
        prompt.get(args_val).await
    }

    /// Returns true if a tool with the given name exists.
    pub async fn has_tool(&self, name: &str) -> bool {
        self.tools.read().await.contains_key(name)
    }

    /// Returns the number of registered tools.
    pub async fn tool_count(&self) -> usize {
        self.tools.read().await.len()
    }
}

// Backward compatibility alias
pub type ToolRegistry = McpRegistry;

impl ToolRegistry {
    // Adapter methods to reuse the old API on the new struct if needed,
    // but the method names mostly match locally (register vs register_tool).
    // Let's add the old methods as aliases to minimize breakage if other code uses them.

    /// Registers a tool in the registry (alias for register_tool)
    pub async fn register(&self, tool: impl Tool + 'static) {
        self.register_tool(tool).await
    }

    /// Registers a boxed tool in the registry (alias for register_tool_boxed)
    pub async fn register_boxed(&self, tool: Box<dyn Tool>) {
        self.register_tool_boxed(tool).await
    }

    /// Unregisters a tool by name (alias for unregister_tool)
    pub async fn unregister(&self, name: &str) -> bool {
        self.unregister_tool(name).await
    }

    /// Calls a tool by name (alias for call_tool)
    pub async fn call(&self, name: &str, ctx: &dyn ToolContext, args: Value) -> Result<Value> {
        self.call_tool(name, ctx, args).await
    }
}
