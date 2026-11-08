# Hyrulor: mapa por fases

La estructura del código se organiza por responsabilidad. Las fases describen
el estado y la evolución del sistema, pero no determinan carpetas físicas.

## Estado

| Fase | Objetivo | Estado |
|---|---|---|
| 0 | Cimientos seguros | Operativa; gate tipado archivado |
| 1 | Agente local y tool-calling | Operativa; validación final de build/tests |
| 2 | Integración de hardware y COSMIC | Núcleo operativo completado; visión y voz locales |
| 3 | Actualización, interfaz y escalado | Applet con autorización y sentidos integrada |

## Fase 0: cimientos seguros

Archivos principales:

```text
Cargo.toml
Cargo.lock
src/main.rs
src/memory_vault.rs
src/tool_sandbox.rs
archive/foundation/sandbox/
archive/foundation/puerta_seguridad/
schema.sql
schema_safe.sql
```

Incluye Tokio, SQLite/FTS5, WAL, socket Unix, Bubblewrap y timeouts. Las
implementaciones antiguas del Safety Gate y del sandbox tipado están
archivadas en `archive/foundation/` y no forman parte del runtime actual.

## Fase 1: agente local

Archivos principales:

```text
src/inference_engine.rs
src/protocol.rs
src/memory_vault.rs
src/tool_sandbox.rs
tools/
training/
```

Incluye RAG, tool-calling con `llama-server --jinja`, validación de
herramientas, ejecución sandboxed, persistencia y pruebas de concurrencia.

El fine-tuning es opcional y no forma parte del camino mínimo operativo.

## Fase 2: integración

Archivos principales:

```text
src/tool_sandbox.rs
tools/
integrations/cosmic-applet/
```

Incluye las herramientas reales de batería/GPU/red, integración COSMIC y
comunicación con el daemon mediante socket Unix. Los módulos de detección y
memoria históricos están archivados.

## Fase 3: evolución

Archivos principales:

```text
integrations/cosmic-applet/
docs/
```

Incluye applet completa (chat, modos de seguridad, formulario de autorización y
comandos de visión/voz), actualización de herramientas, hot-reload, métricas,
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

