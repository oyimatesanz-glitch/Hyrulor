# Hyrulor: mapa por fases

La estructura del código se organiza por responsabilidad. Las fases describen
el estado y la evolución del sistema, pero no determinan carpetas físicas.

## Correspondencia con el whitepaper (`Hyrulor.md`)

El whitepaper numera las fases 0–6; este mapa agrupa el estado interno en 4
etapas. Correlación para no confundirse:

| Whitepaper (`Hyrulor.md`) | Este mapa |
|---|---|
| F0 — Cimientos inmutables y entorno | F0 — cimientos seguros |
| F1 — Seguridad inmutable y detección de hardware | F1 — agente local y tool-calling |
| F2 — Memoria jerárquica, RAG y búsqueda vectorial | F2 — núcleo operativo (memoria + sentidos) |
| F3 — Interfaz COSMIC nativa | F2/F3 — integración COSMIC y escalado |
| F4 — Especialización, entrenamiento y agentes | Motor de Omnisciencia (post-F3, plan A3) |
| F5 — Universalidad, energía y despliegue | Plan A4 (post-Omnisciencia) |
| F6 — Release, documentación y CI/CD | v1.0.0 Genesis (plan A5) |

La planificación detallada por bloques (A0–A5, B0–B1) con criterios de
aceptación vive en `PLAN-MAESTRO.md` (raíz de `Proyectos/HYRULOR/`).

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

