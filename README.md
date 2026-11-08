# 🏛️ Hyrulor: Sistema Operativo Cognitivo Universal

[![Estado](https://img.shields.io/badge/Estado-En_Desarrollo_(Fase_2)-orange.svg)](#estado-del-proyecto)
[![Rust](https://img.shields.io/badge/Core-Rust_2021-black.svg?logo=rust)](#stack-tecnológico)
[![LLM](https://img.shields.io/badge/Inferencia-llama.cpp_|_Vulkan-blue.svg)](#stack-tecnológico)
[![DB](https://img.shields.io/badge/Memoria-SQLite_|_vec0_|_FTS5-blue.svg)](#stack-tecnológico)
[![UI](https://img.shields.io/badge/UI-COSMIC_Desktop-purple.svg)](#stack-tecnológico)

**Hyrulor** no es un simple asistente de chat. Es una **Infraestructura de Inteligencia Cognitiva Distribuida y Local-First**, diseñada para actuar como una capa de abstracción segura entre el usuario y el sistema operativo. Construido bajo los principios de **Zero-Trust**, **Lazy Loading** y **Ejecución Sandboxed**.

---

## ⚠️ Estado del Proyecto

> **Fase Actual: Desarrollo Activo (Fases 1 y 2 en progreso).**  
> El núcleo del daemon (Fase 0) y la interfaz gráfica base (Fase 3) están operativos. Actualmente el cerebro es **Qwen3.5-4B-Instruct** con 5 herramientas esenciales y memoria semántica (RAG).

---

## 🏗️ Arquitectura del Sistema

Hyrulor opera bajo una **arquitectura de enjambre de modelos** (Mixture of Experts), orquestada por un daemon central en Rust:

1. **Router / Orquestador (Rust):** Recibe las peticiones por un *Socket Unix* (`/tmp/omni.sock`), clasifica la intención y decide qué modelo o herramienta invocar.
2. **Capa de Inferencia (llama.cpp + Vulkan):** 
   * *Motor Principal:* `Qwen3.5-4B-Instruct` (Q4_K_M, contexto 8K, acelerado por hardware vía Vulkan).
   * *Motor de Memoria:* `Qwen3-Embedding-0.6B` (generación de vectores para RAG).
3. **Arquitectura de Dos Fases:** 
   * *Fase RAG:* Si la pregunta coincide con el código indexado, el modelo responde **sin** acceso a herramientas del sistema (evitando alucinaciones de hardware).
   * *Fase Herramientas:* Si la pregunta es sobre el hardware/sistema, el modelo utiliza el *Tool-Calling* estricto.
4. **Sandbox de Seguridad:** Toda ejecución de comandos (disco, memoria, procesos, batería y GPU) se ejecuta aislada mediante `bubblewrap`, con permisos mínimos y sin acceso a la red externa.
5. **Memoria Híbrida (SQLite):** Combinación de `FTS5` (búsqueda por texto exacto en conversaciones) y `sqlite-vec` (búsqueda semántica en archivos locales).

---

## 🚀 Características Principales (Implementadas)

*   **Inferencia 100% Local:** Sin dependencias de la nube ni fuga de datos.
*   **Control Unificado:** Script `hyrulor.sh` para gestionar el ciclo de vida de los 8 servicios (start/stop/chat/logs) y cliente interactivo `repl`, sin necesidad de múltiples terminales.
*   **Indexación RAG (Chunking):** Lectura, fragmentación y almacenamiento de archivos locales en una base de datos vectorial para consultas posteriores.
*   **Seguridad de Ejecución:** El *Safety Gate* en Rust valida que el LLM solo invoque herramientas permitidas y valida los argumentos JSON antes de tocar el sistema.

---

## 📋 Hoja de Ruta (Roadmap)

*   [x] **Fase 0:** Daemon asíncrono, SQLite (WAL), Sandbox y Socket Seguro.
*   [x] **Fase 1 (Parcial):** Tool-calling nativo (Disco, Memoria, Procesos, Batería, GPU).
*   [x] **Fase 2 (Parcial):** Memoria Vectorial (`sqlite-vec`) y arquitectura dual-fase.
*   [x] **Fase 3 (Parcial):** Integración con el escritorio COSMIC (Applet Rust).
*   [ ] **Fase 1 (Completar):** Despliegue de las **50 herramientas** (Sistema, Red, Código, Productividad, Seguridad).
*   [ ] **Fase 2 (Completar):** Auto-indexación, *Reranking*, y Memoria Procedimental.
*   [ ] **Fase 4:** Motor de Omnisciencia (ingesta autónoma de conocimiento).

---

## 🛠️ Stack Tecnológico

*   **Lenguaje Core:** Rust (Tokio, Serde, Rusqlite).
*   **Inferencia:** `llama.cpp` (Backend Vulkan).
*   **Base de Datos:** SQLite (WAL, FTS5) + Extensión `sqlite-vec`.
*   **UI:** `libcosmic` (Rust/GTK4) para el entorno de escritorio COSMIC (Pop!_OS).
*   **Sandbox:** `bubblewrap`.

---

## 🚦 Inicio Rápido (Entorno de Desarrollo)

Requisitos: `Rust`, `llama.cpp` compilado con Vulkan, modelos GGUF en `/models`.

```bash
# 1. Arrancar todos los servicios (Motor LLM, Embeddings y Daemon)
./hyrulor.sh start

# 2. Hablar con el asistente (consulta única)
./hyrulor.sh chat "¿Qué hace la función main en mi código?"

# 2b. Cliente interactivo (REPL)
./hyrulor.sh repl

# 3. Ver estado de los servicios
./hyrulor.sh status

# 4. Detener los servicios
./hyrulor.sh stop# Hyrulor

Asistente local Hyrulor para COSMIC Desktop.

Consulta el [mapa de fases](docs/roadmap.md) para conocer la relación entre
los componentes y el whitepaper.

## Estructura

- `src/`: daemon Rust, memoria FTS5, inferencia y sandbox.
- `tools/`: herramientas permitidas para Bubblewrap.
- `integrations/cosmic-applet/`: applet COSMIC.
- `integrations/layerslayer/`: integración de Layerslayer.
- `legacy-go/`: prototipos Go y orquestador auxiliar.

Los modelos GGUF, bases de datos, compilaciones, toolchains y runtimes binarios se mantienen fuera del repositorio mediante `.gitignore`.

## Compilar

```bash
cd /home/oyi/Proyectos/hyrulor
cargo fmt
cargo check
cargo test
cargo build --release
```

## Ejecutar

```bash
cd /home/oyi/Proyectos/hyrulor
HYORY_SOCKET=/tmp/omni.sock \
HYORY_LLM_SOCKET="$HOME/.hyrulor_run/llm.sock" \
HYORY_EMBEDDING_SOCKET="$HOME/.hyrulor_run/emb.sock" \
HYORY_LLM_MODEL=Qwen3.5-4B-Instruct-Q4_K_M \
HYORY_DB="$PWD/hyory.db" \
HYORY_TOOLS="$PWD/tools" \
./target/release/hyory
```

`llama-server` escucha HTTP sobre un socket Unix (sin abrir puertos TCP):

```bash
mkdir -p "$HOME/.hyrulor_run"
./runtime/llama.cpp/build/bin/llama-server \
  -m "$PWD/models/Qwen3.5-4B-Instruct-Q4_K_M.gguf" \
  --host "$HOME/.hyrulor_run/llm.sock" --port 9999 \
  --ctx-size 8192 --n-gpu-layers 99 --jinja
```

### 💾 Paso 3: Guardar en nano
1. Pega el texto.
2. Pulsa **`Ctrl + O`** (la letra O, no el cero) para guardar.
3. Pulsa **`Enter`** para confirmar el nombre del archivo.
4. Pulsa **`Ctrl + X`** para salir.

### 🚀 Paso 4: Subirlo a GitHub

Ahora que el archivo está en tu disco duro (y no ha fallado la terminal), hacemos el commit y el push:

```bash
cd ~/Proyectos/hyrulor

# 1. Añadir el README
git add README.md

# 2. Hacer commit
git commit -m "docs: add formal technical README and project roadmap"

# 3. Traer cambios de GitHub por si acaso y aplicar los tuyos
git pull origin main --rebase

# 4. Subir todo
git push origin main
git push origin --tags
