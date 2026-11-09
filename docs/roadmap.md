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

Además, la memoria persistente (`hyory.db`) se cifra en reposo con
**SQLCipher (AES-256)** vía `rusqlite` `bundled-sqlcipher-vendored-openssl`,
con migración automatizada (`migrate_db`) y verificación (`migrate_db --verify`).

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

## Hyrulor Engine (subsistema visual)

El motor gráfico nativo vive en el repositorio hermano `../hyrulor-engine/`
(100% Rust: `winit` + `wgpu` + `egui` + shaders WGSL, sin WebView) y habla con
este daemon por el mismo socket Unix (`/tmp/omni.sock`, JSON-Lines).

| Hito | Objetivo | Estado |
|---|---|---|
| E1 | Ventana universal (winit) | ✅ |
| E2 | Pipeline básico (wgpu) | ✅ |
| E3 | Orbe 3D (esfera + anillo orbital) | ✅ |
| E4 | Shader holográfico (Fresnel + escaneo + pulso) | ✅ |
| E5 | Chat nativo (egui) | ✅ |
| E6 | Conexión con el daemon (socket Unix) | ✅ |
| E6+ | Autorización humana en la ventana | ✅ |
| E7-E10 | Post-proceso, WASM, móvil, editor | Pendiente |

El orbe codifica el estado del sistema (en línea, pensando, bloqueado, error,
sin conexión) y presenta el formulario de autorización humana directamente en
la ventana nativa.

## Artefactos fuera del repositorio

Los siguientes elementos se mantienen fuera de Git mediante `.gitignore`:

```text
models/
target/
.toolchain/
hyory.db*
runtime/llama.cpp/
```

