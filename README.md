# Hyrulor

Proyecto unificado del asistente local Hyory/Hyrulor.

## Estructura

- `src/`: daemon Rust, memoria FTS5, inferencia y sandbox.
- `tools/`: herramientas permitidas para Bubblewrap.
- `models/`: modelos GGUF y artefactos de inferencia.
- `runtime/llama.cpp/`: runtime local de llama.cpp.
- `runtime/goinfer-serve`: servidor Go de inferencia.
- `legacy-go/`: prototipos Go y orquestador auxiliar.
- `integrations/layerslayer/`: integración completa de Layerslayer.
- `archives/`: duplicados, fuentes originales y workspace histórico.
- `hyory.db`: base de memoria activa.

## Compilar

```bash
cd /home/oyi/hyrulor
cargo fmt
cargo check
cargo test
cargo build --release
```

## Ejecutar

```bash
cd /home/oyi/hyrulor
HYORY_SOCKET=/tmp/omni.sock \
HYORY_LLM_SOCKET=/tmp/llm.sock \
HYORY_LLM_MODEL=Qwen2.5-3B-Instruct-Q5_K_M \
HYORY_DB="$PWD/hyory.db" \
HYORY_TOOLS="$PWD/tools" \
./target/release/hyory
```

Los recursos no relacionados con Hyrulor permanecen fuera del proyecto.
