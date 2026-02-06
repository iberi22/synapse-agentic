//! MCP Server implementation (JSON-RPC 2.0 over Stdio).

use std::io::{self, BufRead, Write};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{info, error, debug};

use super::registry::McpRegistry; // Used to be ToolRegistry
use super::tool::ToolContext;

/// JSON-RPC 2.0 Request
#[derive(Deserialize, Debug)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Option<Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Serialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<RpcError>,
    id: Option<Value>,
}

/// JSON-RPC 2.0 Error
#[derive(Serialize, Debug)]
struct RpcError {
    code: i32,
    message: String,
}

/// MCP Server configuration
pub struct McpServerConfig {
    /// Server name for identification
    pub name: String,
    /// Server version
    pub version: String,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            name: "synapse-agentic".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// MCP Server that handles JSON-RPC 2.0 over Stdio.
///
/// # Example
///
/// ```rust,no_run
/// use synapse_agentic::mcp::{McpServer, ToolRegistry, EmptyContext};
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() {
///     let registry = Arc::new(ToolRegistry::new());
///     let ctx = Arc::new(EmptyContext);
///
///     let server = McpServer::new(registry, ctx);
///     server.run_stdio().await.unwrap();
/// }
/// ```
pub struct McpServer<C: ToolContext + 'static> {
    registry: Arc<McpRegistry>,
    context: Arc<C>,
    config: McpServerConfig,
}

impl<C: ToolContext + 'static> McpServer<C> {
    /// Creates a new MCP server.
    pub fn new(registry: Arc<McpRegistry>, context: Arc<C>) -> Self {
        Self {
            registry,
            context,
            config: McpServerConfig::default(),
        }
    }

    /// Creates a new MCP server with custom configuration.
    pub fn with_config(registry: Arc<McpRegistry>, context: Arc<C>, config: McpServerConfig) -> Self {
        Self {
            registry,
            context,
            config,
        }
    }

    /// Runs the MCP server on Stdio (stdin/stdout).
    ///
    /// This is the standard transport for MCP.
    pub async fn run_stdio(self) -> anyhow::Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        info!(server = %self.config.name, "MCP Server starting on stdio");

        for line in stdin.lock().lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            debug!(request = %line, "Received MCP request");

            let req: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    error!(error = %e, "Failed to parse JSON-RPC request");
                    continue;
                }
            };

            let response = self.handle_request(req).await;
            let response_str = serde_json::to_string(&response)?;

            debug!(response = %response_str, "Sending MCP response");

            writeln!(stdout, "{}", response_str)?;
            stdout.flush()?;
        }

        info!("MCP Server shutting down");
        Ok(())
    }

    async fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let result = match req.method.as_str() {
            "initialize" => self.handle_initialize().await,
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tools_call(req.params).await,
            "resources/list" => self.handle_resources_list().await,
            "resources/read" => self.handle_resources_read(req.params).await,
            "prompts/list" => self.handle_prompts_list().await,
            "prompts/get" => self.handle_prompts_get(req.params).await,
            "ping" => Ok(json!({ "pong": true })),
            _ => Err(RpcError {
                code: -32601,
                message: format!("Method not found: {}", req.method),
            }),
        };

        match result {
            Ok(val) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(val),
                error: None,
                id: req.id,
            },
            Err(err) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(err),
                id: req.id,
            },
        }
    }

    async fn handle_initialize(&self) -> Result<Value, RpcError> {
        Ok(json!({
            "protocolVersion": "0.1.0",
            "capabilities": {
                "tools": {},
                "resources": {},
                "prompts": {}
            },
            "serverInfo": {
                "name": self.config.name,
                "version": self.config.version
            }
        }))
    }

    async fn handle_tools_list(&self) -> Result<Value, RpcError> {
        let tools = self.registry.list_tools().await;
        Ok(json!({ "tools": tools }))
    }

    async fn handle_tools_call(&self, params: Option<Value>) -> Result<Value, RpcError> {
        let params = params.ok_or_else(|| RpcError {
            code: -32602,
            message: "Missing params".to_string(),
        })?;

        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError {
                code: -32602,
                message: "Missing tool name".to_string(),
            })?;

        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or(Value::Null);

        match self.registry.call_tool(name, self.context.as_ref(), args).await {
            Ok(result) => Ok(json!({
                "content": [{
                    "type": "text",
                    "text": result.to_string()
                }]
            })),
            Err(e) => Err(RpcError {
                code: -32603,
                message: e.to_string(),
            }),
        }
    }

    async fn handle_resources_list(&self) -> Result<Value, RpcError> {
        let resources = self.registry.list_resources().await;
        Ok(json!({ "resources": resources }))
    }

    async fn handle_resources_read(&self, params: Option<Value>) -> Result<Value, RpcError> {
        let params = params.ok_or_else(|| RpcError {
            code: -32602,
            message: "Missing params".to_string(),
        })?;

        let uri = params
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError {
                code: -32602,
                message: "Missing resource uri".to_string(),
            })?;

        match self.registry.read_resource(uri).await {
            Ok(text) => {
                Ok(json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "text/plain", // TODO: Get actual mime type from registry if needed
                        "text": text
                    }]
                }))
            }
            Err(e) => Err(RpcError {
                code: -32603,
                message: e.to_string(),
            }),
        }
    }

    async fn handle_prompts_list(&self) -> Result<Value, RpcError> {
        let prompts = self.registry.list_prompts().await;
        Ok(json!({ "prompts": prompts }))
    }

    async fn handle_prompts_get(&self, params: Option<Value>) -> Result<Value, RpcError> {
        let params = params.ok_or_else(|| RpcError {
            code: -32602,
            message: "Missing params".to_string(),
        })?;

        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError {
                code: -32602,
                message: "Missing prompt name".to_string(),
            })?;

        let arguments_json = params
            .get("arguments")
            .cloned()
            .unwrap_or(json!({}));

        // Convert arguments to HashMap<String, String>
        let mut arguments = std::collections::HashMap::new();
        if let Some(obj) = arguments_json.as_object() {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    arguments.insert(k.clone(), s.to_string());
                }
            }
        }

        match self.registry.get_prompt(name, arguments).await {
            Ok(result) => Ok(json!(result)),
            Err(e) => Err(RpcError {
                code: -32603,
                message: e.to_string(),
            }),
        }
    }
}

