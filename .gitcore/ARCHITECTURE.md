# Architecture - synapse-agentic

## CRITICAL DECISIONS - READ FIRST

> ⚠️ STOP! Before implementing ANY feature, verify against this table.

| # | Category | Decision | Rationale | NEVER Use |
|---|----------|----------|-----------|-----------|
| 1 | Migration | Gradual TS→Rust | No downtime | Big bang rewrite |
| 2 | State | GitHub Issues + SurrealDB | Persistent | Local files only |
| 3 | Agents | Clawd (TS) ↔ Jules (Rust) | Hybrid | Single agent |
| 4 | API | REST + MCP | Extensibility | Monolithic |

---

## Stack
- **Language:** Rust (core), TypeScript (bridge layer)
- **Framework:** Actix Web, Tokio
- **Database:** SurrealDB (vector + relational)
- **Agents:** Clawd (MiniMax M2.1), Jules (GitCore)

## Architecture Layers

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   TS Layer      │────▶│   Rust Layer    │────▶│   Data Layer    │
│ clawdbot-new    │     │ synapse-agentic │     │ SurrealDB       │
│ - Channels      │     │ - MCP Server    │     │ - Memory        │
│ - Gateway       │     │ - Decision Eng  │     │ - Documents     │
│ - Skills        │     │ - Agent Core    │     │ - Vectors       │
└─────────────────┘     └─────────────────┘     └─────────────────┘
         │                      │                       │
         └──────────────────────┼───────────────────────┘
                                ▼
                    ┌─────────────────────┐
                    │   GitHub Issues     │
                    │   (State + Tasks)   │
                    └─────────────────────┘
```

## Key Components

### 1. MCP Server (Model Context Protocol)
- Expose Rust functions to TS layer
- Tool discovery and execution
- JSON-RPC 2.0 interface

### 2. Decision Engine
- Consensus voting between LLMs
- MiniMax M2.1, Kimi K2.5, Qwen Coder
- Weighted scoring system

### 3. Memory Store
- Short-term: Working memory (in-memory)
- Long-term: SurrealDB (persistent)
- Semantic: Vector embeddings

## Migration Phases

| Phase | Focus | Components |
|-------|-------|------------|
| 1 | Bridge | TS↔Rust communication |
| 2 | Core | Memory + Decision Engine |
| 3 | Agents | Jules orchestration |
| 4 | Integration | Full migration |

---
*Last updated: 2026-02-01*
