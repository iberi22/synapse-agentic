# Informe de Viabilidad: Migración a `rust-adk` vs Integración de Lógicas

## 1. Resumen Ejecutivo
El objetivo de este informe es analizar la viabilidad de migrar el framework `synapse-agentic` hacia la capa proporcionada por `rust-adk` (Agent Development Kit de Inference Gateway), o de manera alternativa, extraer e integrar sus lógicas dentro de la arquitectura actual.

La conclusión principal es que **una migración completa (mover toda la base a la capa de rust-adk) no es recomendable**, ya que implicaría reescribir la arquitectura basada en Actores y perder el enfoque nativo en MCP. En su lugar, **la opción más viable y beneficiosa es un enfoque híbrido: extraer e integrar las lógicas de `rust-adk`**. Esto dotaría a `synapse-agentic` de compatibilidad con el protocolo A2A (Agent-to-Agent), expandiendo su interoperabilidad sin perder sus robustas características actuales (Decisiones Multi-LLM, Memoria, Seguridad y Actores).

## 2. Análisis de Arquitecturas

### 2.1 Estado Actual: `synapse-agentic`
- **Modelo de Concurrencia:** Basado en Actores (`Hive`, Traits `Agent`), muy eficiente para manejar estados locales, supervisión y mensajería asíncrona.
- **Protocolos:** Fuerte enfoque en MCP (Model Context Protocol), ideal para integración con asistentes (Claude, Cursor) y sistemas locales.
- **Características Únicas:**
  - `DecisionEngine` para consenso entre múltiples LLMs (DeepSeek, Grok, Gemini, etc.).
  - Componentes avanzados modulares: `parser` (self-healing), `security` (redacción de PII), `compaction`, y `persistence` (SurrealDB, Postgres, pgvector).

### 2.2 La Propuesta: `rust-adk`
- **Enfoque:** Construcción de servidores y clientes bajo el protocolo estándar **A2A** (Agent-to-Agent).
- **Modelo de Arquitectura:** Basado en un servidor HTTP (`A2AServer`) con manejadores de tareas (`TaskProcessor`) y peticiones REST/Streaming.
- **Características Destacadas:**
  - Comunicación inter-agente estandarizada (A2A).
  - Health checks y descubrimiento de agentes mediante `AgentCards` (`.well-known/agent.json`).
  - Notificaciones Push (Webhooks) para cambios de estado en tareas de larga duración.
  - Autenticación segura (OIDC/OAuth2).

## 3. Escenario A: Migración Completa (Mover todo a `rust-adk`)

**Concepto:** Reemplazar el núcleo de `synapse-agentic` (el `Hive` y los `Agent` traits) por el `A2AServer` y el `AgentBuilder` de `rust-adk`.

**Viabilidad:** 🔴 **Baja / No recomendada.**

**Razones:**
1. **Pérdida del Modelo de Actores:** `rust-adk` es fundamentalmente un servidor web enfocado en enrutar tareas, no un framework de actores. Perderíamos la flexibilidad del envío de mensajes asíncronos tipados, el aislamiento de estado y el supervisor `Hive`.
2. **Conflicto de Protocolos Core:** `synapse-agentic` es "MCP-first". Mover toda la capa base a `rust-adk` (que es "A2A-first") obligaría a adaptar MCP como un parche secundario, lo cual rompe la visión principal del proyecto actual.
3. **Acoplamiento Fuerte con Inference Gateway SDK:** `rust-adk` depende fuertemente de su propio SDK para el manejo de herramientas y LLMs. En `synapse-agentic` ya tenemos un `ToolRegistry` robusto agnóstico basado en `schemars` y adaptadores propios. Movernos obligaría a reescribir integraciones ya funcionales.

## 4. Escenario B: Integración de Lógicas Extraídas (Enfoque Híbrido)

**Concepto:** Mantener la arquitectura actual de `synapse-agentic` y añadir características de `rust-adk` como un módulo o dependencia opcional (ej. habilitado vía feature flag `a2a`).

**Viabilidad:** 🟢 **Alta / Muy recomendada.**

**Lógicas a Extraer e Integrar:**
1. **Soporte Dual MCP + A2A:**
   - Desarrollar un adaptador que permita exponer un `Agent` del `Hive` a través del `A2AServer`. Así, un agente puede servir a un humano vía MCP y a otros sistemas autónomos vía A2A simultáneamente.
2. **Cliente A2A como "Tool":**
   - Extraer el `A2AClient` y envolverlo en el trait `Tool` de `synapse-agentic`. Esto permitiría que nuestros agentes deleguen tareas dinámicamente a agentes A2A remotos como si estuvieran llamando a una función local.
3. **Descubrimiento (AgentCards) y Health Checks:**
   - Integrar la generación estandarizada de metadatos de `rust-adk`. Que el `Hive` sea capaz de generar automáticamente un archivo `agent.json` para participar en ecosistemas A2A.
4. **Notificaciones Push y Ciclo de Vida de Tareas:**
   - Adoptar la lógica de webhooks y seguimiento de estado de tareas de `rust-adk` como un plugin o middleware en nuestro módulo de canales (`channels`).

## 5. Pros y Contras del Enfoque Híbrido Recomendado

| Pros | Contras |
|------|---------|
| **Máxima Interoperabilidad:** Los agentes hablan con humanos/asistentes (MCP) y con otros agentes (A2A). | **Complejidad de Mantenimiento:** Se debe soportar y probar la convivencia de ambos protocolos. |
| **Sin Regresiones:** El código existente (DecisionEngine, Actores, Memoria) permanece intacto. | **Adaptación de Interfaces:** Habrá que construir mappers entre las estructuras de Tool de `synapse-agentic` y las de `rust-adk`. |
| **Adopción Opt-in:** Si los usuarios no necesitan A2A, no pagan el coste (gracias a los features de Cargo). | |

## 6. Conclusión y Recomendación Final

**Conclusión:** Es inviable y contraproducente migrar la capa fundacional de `synapse-agentic` a `rust-adk`. Sin embargo, **es altamente viable y estratégico utilizar lógicas extraídas de `rust-adk`** para enriquecer las capacidades de comunicación del framework.

**Plan de Acción Sugerido:**
1. Añadir `inference-gateway-adk` como dependencia opcional en `Cargo.toml` (`[features] a2a = ["dep:inference-gateway-adk"]`).
2. Crear un nuevo módulo `src/a2a/` en `synapse-agentic`.
3. Implementar un `A2AAdapter` que intercepte peticiones A2A y las enrute como mensajes tipados hacia el `EventBus` del `Hive`.
4. Implementar una `A2ADelegationTool` nativa de `synapse-agentic` que use el cliente de `rust-adk` para interactuar con agentes remotos.
