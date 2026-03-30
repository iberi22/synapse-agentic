# Requisitos Funcionales

**Proyecto:** synapse-agentic  
**Fecha:** 2026-03-21

---

## Módulos Detectados por GitCore

- channels
- compaction
- decision
- framework
- mcp
- memory
- networking
- primitives
- protocol

---

## Requisitos Funcionales

| ID | Descripción | Prioridad | Estado |
|----|-------------|-----------|--------|
| SRC-FUN-001 | Agent Framework - Actor-based agents con mensajes tipados y supervisión | Alta | Implementado |
| SRC-FUN-002 | MCP Server - Model Context Protocol para integración AI assistants | Alta | Implementado |
| SRC-FUN-003 | Multi-LLM Support - OpenRouter, DeepSeek, Gemini, Grok con consensus voting | Alta | Implementado |
| SRC-FUN-004 | Tool System - Definir capacidades como JSON-Schema validated tools | Media | Implementado |
| SRC-FUN-005 | Memory Abstraction - Almacenes de memoria pluggable de corto y largo plazo | Media | Implementado |

---

## Casos de Uso

### UC-001: Crear Agente Simple

**Actor:** Desarrollador  
**Descripción:** Crear un agente básico con capacidades predefined  
**Precondiciones:** Cargo.toml configurado con synapse-agentic  
**Flujo Principal:**
1. Definir herramientas en JSON Schema
2. Crear agente con actor model
3. Configurar supervisor
4. Iniciar agente

---

## Reglas de Negocio

- Agentes deben usar mensajes tipados
- MCP server requerido para integración externa
- Consensus voting para decisiones multi-LLM

---

*Actualizado: 2026-03-21*