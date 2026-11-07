# Hyrulor: mapa por fases

La estructura del código se organiza por responsabilidad. Las fases describen
el estado y la evolución del sistema, pero no determinan carpetas físicas.

## Estado

| Fase | Objetivo | Estado |
|---|---|---|
| 0 | Cimientos seguros | Completada |
| 1 | Agente local y tool-calling | Operativa; validación final de build/tests |
| 2 | Integración de hardware y COSMIC | Núcleo operativo completado; applet en integración |
| 3 | Actualización, interfaz y escalado | Pendiente |

## Fase 0: cimientos seguros

Archivos principales:

```text
Cargo.toml
Cargo.lock
src/main.rs
src/memory_vault.rs
src/tool_sandbox.rs
src/foundation/sandbox/
src/foundation/puerta_seguridad/
schema.sql
schema_safe.sql
```

Incluye Tokio, SQLite/FTS5, WAL, socket Unix, Bubblewrap, timeouts y
controles de seguridad.

## Fase 1: agente local

Archivos principales:

```text
src/inference_engine.rs
src/protocol.rs
src/memory_vault.rs
src/tool_sandbox.rs
tools/
training/
tests/
```

Incluye RAG, tool-calling con `llama-server --jinja`, validación de
herramientas, ejecución sandboxed, persistencia y pruebas de concurrencia.

El fine-tuning es opcional y no forma parte del camino mínimo operativo.

## Fase 2: integración

Archivos principales:

```text
src/foundation/inicializacion/
src/foundation/memoria/
integrations/cosmic-applet/
```

Incluye detección de hardware, integración COSMIC y comunicación con el
daemon mediante socket Unix.

## Fase 3: evolución

Pendientes previstos:

```text
integrations/cosmic-applet/
docs/
```

Incluye applet completa, actualización de herramientas, hot-reload, métricas,
benchmarking y optimización continua.

## Artefactos fuera del repositorio

Los siguientes elementos se mantienen fuera de Git mediante `.gitignore`:

```text
models/
target/
.toolchain/
hyory.db*
runtime/llama.cpp/
```

