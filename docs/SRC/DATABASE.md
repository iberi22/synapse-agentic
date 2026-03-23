# Modelo de Datos

**Proyecto:** synapse-agentic  
**Fecha:** 2026-03-21

---

## Estructuras de Datos Principales

### AgentMessage

```rust
pub struct AgentMessage {
    pub id: Uuid,
    pub sender: AgentId,
    pub recipient: AgentId,
    pub payload: MessagePayload,
    pub timestamp: DateTime<Utc>,
}
```

### Tool

```rust
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub handler: Box<dyn ToolHandler>,
}
```

### MemoryEntry

```rust
pub struct MemoryEntry {
    pub key: String,
    pub value: Value,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
```

---

## Diagrama de Relaciones

```
Agent -> Message -> Channel
Agent -> Tool -> MCP Server
Agent -> Memory -> MemoryStore
Agent -> LLMProvider -> Consensus Voting
```

---

*Actualizado: 2026-03-21*