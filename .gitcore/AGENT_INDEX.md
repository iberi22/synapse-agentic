# Agent Index - synapse-agentic

## Agents Disponibles

| Agente | Rol | Modelo/Stack | Estado |
|--------|-----|--------------|--------|
| **Jules** | Orquestador | GitCore CLI v3.5.1 | ✅ Activo |
| **Clawd** | Principal | MiniMax M2.1 | ✅ Activo |
| **Gemini CLI** | Investigador | v0.26.0 | ⚪ Disponible |
| **Qwen Coder** | Multilingüe | API | ⚪ Pendiente |

## Workflow GitCore

```
Clawd (Telegram) → GitHub Issue → Jules (gc task start) → gc finish → gc report → Clawd
```

## Comandos Rapidos

```bash
# Ver tareas asignadas
gc issue list --assigned-to-me

# Ver todas las tareas
gc issue list

# Ver PRs
gc pr list

# Iniciar tarea
gc task start "Nombre de tarea"

# Finalizar tarea
gc finish

# Reporte
gc report
```

## Issues Activos

| # | Título | Asignado | Estado |
|---|--------|----------|--------|
| #34 | Revisar PRs dependencias opentelemetry | Jules | ⏳ Pendiente |
| #35 | Documentar arquitectura clawdbot-new | Jules | ⏳ Pendiente |
| #33 | Implementar persistencia con SurrealDB | backend | 🔄 En Progreso |
| #25 | Wire up Main Application and API Server | jules | ⏳ Pendiente |

---
*Generado: 2026-02-01*
