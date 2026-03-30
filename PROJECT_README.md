# PROJECT_README.md

> **IMPORTANT:** Replace all `<!-- QUESTION: ... -->` blocks with actual answers from Bel.

## Overview

**Synapse Agentic** is a modern Rust framework for building AI-native agentic systems. It provides an actor-based agent model with typed messages, MCP (Model Context Protocol) server support, multi-LLM consensus voting, and pluggable memory stores.

## Problem Statement

Building production AI agents requires handling complex concerns: message routing, supervision, tool definition, memory management, and multi-model orchestration. Synapse Agentic provides a clean, composable framework for this.

## Target Users

- Rust developers building AI agent systems
- Teams needing MCP integration (VS Code, Cursor, Claude)
- Applications requiring multi-LLM consensus

## Tech Stack

| Component | Technology |
|-----------|------------|
| Language | Rust |
| Async Runtime | Tokio |
| MCP | Model Context Protocol |
| LLM Routing | OpenRouter, DeepSeek, Gemini, Grok |
| Serialization | JSON-Schema validation |

## Key Modules

| Module | Purpose |
|--------|---------|
| `framework/` | Agent runtime, Hive, EventBus |
| `mcp/` | MCP server implementation |
| `tools/` | Tool system with JSON-Schema |
| `memory/` | Short-term and long-term memory |

## Quick Start

```toml
# Add to Cargo.toml
synapse-agentic = "0.1"
```

```rust
// See full examples in docs/
```

## Status

**Active Development** - Available on crates.io

## Business Model

Open source (MIT licensed)

## Related Projects

| Project | Relationship |
|---------|-------------|
| synapse-enterprise | Monorepo containing synapse-agentic |
| synapse-internet-evolve | Bio-mimetic memory research |
| GitCore | Protocol for agent coordination |

---

*Questions marked with `<!-- QUESTION: ... -->` need answers from Bel.*
