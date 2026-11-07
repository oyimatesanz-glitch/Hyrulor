# Hyrulor

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
HYORY_LLM_ADDR=127.0.0.1:8082 \
HYORY_LLM_MODEL=Qwen2.5-3B-Instruct-Q5_K_M \
HYORY_DB="$PWD/hyory.db" \
HYORY_TOOLS="$PWD/tools" \
./target/release/hyory
```

`llama-server` usa HTTP TCP, no sockets Unix:

```bash
./runtime/llama.cpp/build/bin/llama-server \
  -m "$PWD/models/Qwen2.5-3B-Instruct-Q5_K_M.gguf" \
  --host 127.0.0.1 --port 8082 --ctx-size 2048 --n-gpu-layers 99 --jinja
```
