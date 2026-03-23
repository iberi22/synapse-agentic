# Interfaces de Sistema

**Proyecto:** synapse-agentic  
**Fecha:** 2026-03-21

---

## Interfaces Externas

### MCP Server Interface

```rust
// Integración con AI assistants (Claude, GPT, etc.)
pub trait MCPServer {
    fn register_tool(&mut self, tool: Tool) -> Result<()>;
    fn send_message(&mut self, msg: AgentMessage) -> Result<Response>;
}
```

### LLM Interface

```rust
pub trait LLMProvider {
    async fn complete(&self, prompt: &str) -> Result<String>;
    async fn consensus_vote(&self, options: Vec<String>) -> Result<String>;
}
```

### Memory Interface

```rust
pub trait MemoryStore {
    async fn get(&self, key: &str) -> Result<Option<Value>>;
    async fn set(&mut self, key: &str, value: Value) -> Result<()>;
    async fn forget(&mut self, key: &str) -> Result<()>;
}
```

---

## API Endpoints

- MCP Protocol endpoints para tools
- HTTP API para integración con agentes externos

---

*Actualizado: 2026-03-21*