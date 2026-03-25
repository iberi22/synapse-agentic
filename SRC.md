# SRC.md - Synapse Agentic

> Documentación de análisis de estructura de proyecto

## Información General

| Campo | Valor |
|-------|-------|
| **Nombre** | synapse-agentic |
| **Tipo** | Framework Rust |
| **Descripción** | Framework moderno en Rust para construir sistemas agenticos AI-native con soporte MCP |
| **Lenguaje** | Rust |
| **Último análisis** | 2026-03-16 |

## Estructura

```
synapse-agentic/
├── src/              # Código fuente principal
├── examples/         # Ejemplos de uso
├── Cargo.toml        # Dependencias Rust
├── README.md         # Documentación principal
└── LICENSE-*        # Licencias
```

## Módulos Detectados

- channels
- compaction
- decision
- framework
- mcp
- (y más...)
- Sistema de herramientas
- Memoria de corto/largo plazo
- Soporte Multi-LLM (OpenRouter, DeepSeek, Gemini, Grok)
- Integración MCP (Model Context Protocol)

## Características Principales

- 🤖 Agent Framework basado en actores
- 🔌 Servidor MCP para integración con Claude, GPT
- 🧠 Soporte Multi-LLM con consenso voting
- 📦 Sistema de herramientas JSON-Schema
- 💾 Abstracción de memoria
- 🚀 Async-first con Tokio

## Uso

```toml
[dependencies]
synapse-agentic = "0.1"
```

## Estado

✅ Proyecto activo - Mantenido por Southwest AI Labs

*Última actualización: 2026-03-17*
