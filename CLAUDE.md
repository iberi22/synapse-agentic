# CLAUDE.md - Template para Proyectos

> Template base para crear archivos CLAUDE.md en proyectos de SWAL.
> Adaptado de mejores prácticas de Claude Code y GitCore Protocol.

## Proyecto: synapse-agentic

**Descripción:** Proyecto en Rust
**Stack:** Rust
**Tipo:** rust (library/cli/webapp/api/agent/script)

---

## 🚀 Quick Start

```bash
# Instalar dependencias
cargo test

# Ejecutar / Build / Dev server
cargo run

# Tests
{TEST_COMMAND}
```

## 📁 Estructura del Proyecto

```
{src_or_main_folder}/
├── {MODULE_OR_FEATURE_1}/      # Descripción
├── {MODULE_OR_FEATURE_2}/      # Descripción
└── {CONFIG_FILES}
```

## 🔧 Comandos Principales

| Comando | Descripción |
|---------|-------------|
| `{dev_command}` | Iniciar desarrollo |
| `{build_command}` | Build de producción |
| `{test_command}` | Ejecutar tests |
| `{lint_command}` | Linting y formateo |

## 🏗️ Arquitectura

# Architecture - synapse-agentic  ## CRITICAL DECISIONS - READ FIRST  > ⚠️ STOP! Before implementing ANY feature, verify against this table.  | # | Category | Decision | Rationale | NEVER Use | |---|----------|----------|-----------|-----------| | 1 | Migration | Gradual TS→Rust | No downtime | Big bang rewrite | | 2 | State | GitHub Issues + SurrealDB | Persistent | Local files only |...

### Patrones Clave
- {PATTERN_1}: {DESCRIPTION}
- {PATTERN_2}: {DESCRIPTION}

## 📋 Convenciones de Desarrollo

### Git Workflow (GitCore Protocol)
- Issues: `.github/issues/` → sincroniza con GitHub
- Commits: `type(scope): description #issue`
- Branches: `type/short-description-#issue`

### Código
- {LINT_RULES}
- {CODING_STANDARD}

## 🔗 Recursos

- Documentación: {DOCS_URL}
- Repo: {REPO_URL}
- Issues: {ISSUES_URL}

## 📝 Notas para Agentes

- Leer `.gitcore/ARCHITECTURE.md` antes de trabajar
- Mantener stateless pattern (GitHub Issues para estado)
- Usar Claude Code para tareas de código

---

*Última actualización: 2026-03-21*
*Generado automáticamente por SWAL Agent System*
