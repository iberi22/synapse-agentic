# 🧠 Synapse Agentic

[![Crates.io](https://img.shields.io/crates/v/synapse-agentic.svg)](https://crates.io/crates/synapse-agentic)
[![Documentation](https://docs.rs/synapse-agentic/badge.svg)](https://docs.rs/synapse-agentic)
[![License](https://img.shields.io/crates/l/synapse-agentic.svg)](LICENSE-MIT)

**A modern Rust framework for building AI-native agentic systems with MCP support.**

## ✨ Features

- 🤖 **Agent Framework** - Actor-based agents with typed messages and supervision
- 🔌 **MCP Server** - Model Context Protocol for AI assistant integration (Claude, GPT, etc.)
- 🧠 **Multi-LLM Support** - OpenRouter, DeepSeek, Gemini, Grok with consensus voting
- 📦 **Tool System** - Define capabilities as JSON-Schema validated tools
- 💾 **Memory Abstraction** - Pluggable short-term and long-term memory stores
- 🚀 **Async-First** - Built on Tokio for maximum performance

## 🚀 Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
synapse-agentic = "0.1"
```

### Create a Simple Agent

```rust
use synapse_agentic::prelude::*;

#[derive(Debug)]
enum MyMessage {
    Greet(String),
    Shutdown,
}

struct GreeterAgent;

#[async_trait]
impl Agent for GreeterAgent {
    type Input = MyMessage;

    fn name(&self) -> &str { "Greeter" }

    async fn handle(&mut self, msg: Self::Input) -> anyhow::Result<()> {
        match msg {
            MyMessage::Greet(name) => println!("Hello, {}!", name),
            MyMessage::Shutdown => println!("Goodbye!"),
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let mut hive = Hive::new();
    let handle = hive.spawn(GreeterAgent);

    handle.send(MyMessage::Greet("World".into())).await.unwrap();

    hive.shutdown().await;
}
```

### Define MCP Tools

```rust
use synapse_agentic::mcp::{Tool, ToolContext};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Deserialize, JsonSchema)]
struct SearchArgs {
    query: String,
    limit: Option<u32>,
}

struct SearchTool;

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &str { "search" }
    fn description(&self) -> &str { "Search the knowledge base" }

    fn parameters(&self) -> serde_json::Value {
        schemars::schema_for!(SearchArgs).into()
    }

    async fn call(&self, ctx: &dyn ToolContext, args: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let args: SearchArgs = serde_json::from_value(args)?;
        // Your search logic here
        Ok(serde_json::json!({ "results": [] }))
    }
}
```

### Multi-LLM Consensus

```rust
use synapse_agentic::decision::{DecisionEngine, DecisionContext};

let engine = DecisionEngine::builder()
    .with_deepseek(std::env::var("DEEPSEEK_API_KEY")?)
    .with_grok(std::env::var("GROK_API_KEY")?)
    .build();

let context = DecisionContext::new("business")
    .with_summary("Should we approve this expense?")
    .with_data(serde_json::json!({ "amount": 5000, "category": "equipment" }));

let decision = engine.decide(&context).await?;
println!("Decision: {:?} (confidence: {})", decision.action, decision.confidence);
```

## 📦 Module Overview

| Module | Description |
|--------|-------------|
| `framework` | Core Agent trait, Hive supervisor, EventBus |
| `mcp` | MCP Server, Tool trait, JSON-RPC handler |
| `decision` | LLM providers, Skills, Consensus engine |
| `prelude` | Convenient re-exports |

## 🔧 Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `mcp` | ✅ | MCP Server support |
| `llm-providers` | ✅ | All LLM integrations |
| `openrouter` | ✅ | OpenRouter API |
| `deepseek` | ✅ | DeepSeek API |
| `gemini` | ✅ | Google Gemini (OAuth) |
| `grok` | ✅ | xAI Grok API |
| `mcp-http` | ❌ | HTTP transport for MCP |
| `full` | ❌ | All features |

## 📄 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## 🤝 Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
