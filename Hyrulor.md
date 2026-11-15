# HYRULOR: Documento Maestro Integral

## Sistema Operativo Cognitivo Universal, Local, Seguro y Nativo para COSMIC

**Versión del documento:** 1.0.0-rc1  
**Nombre en clave:** Genesis  
**Fecha:** 2026-11-09  
**Propietario del proyecto:** Oscar Matesanz  
**Clasificación:** Confidencial / Zero-Trust / Local-First
**Hardware base objetivo:** Lenovo Yoga Slim 7 14AGP11, AMD Ryzen AI 7 445 (Zen 5, NPU ~50 TOPS), Radeon 840M integrada (RDNA 3.5, gfx1153), 32 GB LPDDR5X 8000 MT/s
**Entorno objetivo:** Pop!_OS 24.04 (Ubuntu 24.04 LTS) con COSMIC Desktop, Wayland
**Stack principal:** Rust 2021+, SQLite, SQLCipher, FTS5, sqlite-vec, libcosmic, wgpu, goinfer/llama.cpp  

---

## 0. Resumen Ejecutivo

Hyrulor no es un chatbot. No es una interfaz web empaquetada. No es un asistente dependiente de la nube. Hyrulor es una **infraestructura cognitiva local**, un **sistema operativo mental distribuido** que orquesta modelos pequeños, herramientas del sistema, memoria persistente, RAG vectorial, agentes especializados y una interfaz nativa de COSMIC.

El objetivo es construir un asistente personal que sea:

- **Local:** todo ocurre en la máquina del usuario.
- **Privado:** sin telemetría, sin subida de documentos a servicios externos.
- **Hiperligero:** núcleo por debajo de 100 MB de RAM en reposo, sin contar modelos cargados en memoria.
- **Universal:** capaz de ejecutarse en ThinkPads antiguos, portátiles modernos, CPUs AMD/Intel, iGPUs Radeon Vega, GPUs NVIDIA y, opcionalmente, entornos contenerizados o MicroVM.
- **Seguro por diseño:** las restricciones éticas y de seguridad no son prompts, sino código compilado, tipos fuertes, listas blancas, sandboxing y auditoría.
- **Sin límite práctico de memoria conversacional:** la ventana del modelo puede ser finita, pero la memoria del sistema es ilimitada en disco mediante SQLite, FTS5, sqlite-vec y compactación jerárquica.
- **Nativo en COSMIC:** el applet principal se construye con `libcosmic`, sin WebView, sin Electron, sin Tauri con navegador embebido.
- **Modular:** el núcleo puede evolucionar hacia un motor gráfico propio, Hyrulor Engine, para interfaces holográficas, orbes 3D, shaders, efectos de cristal y visualización avanzada.

La visión es ambiciosa, pero el documento la convierte en ingeniería ejecutable. Se elimina la confusión entre “quiero un asistente universal” y “voy a construir un monolito imposible”. Hyrulor se diseña como un **enjambre de agentes especializados** orquestado localmente.

---

## 1. Manifiesto del Proyecto

### 1.1. Qué es Hyrulor

Hyrulor es un **Sistema Operativo Cognitivo Universal**. Su función no es solamente responder texto, sino:

1. Entender intención del usuario.
2. Clasificar la petición.
3. Consultar hardware, energía, procesos, archivos, red o conocimiento.
4. Aplicar políticas de seguridad antes de ejecutar cualquier acción.
5. Recuperar memoria de largo plazo.
6. Usar modelos locales especializados.
7. Sintetizar una respuesta unificada.
8. Registrar auditoría cifrada.
9. Aprender de forma controlada.
10. Integrarse nativamente con COSMIC Desktop.

### 1.2. Qué no es Hyrulor

Hyrulor no debe confundirse con:

- Un wrapper de API comercial.
- Una aplicación Electron.
- Una web app con WebView.
- Un modelo monolítico gigante que intente hacerlo todo.
- Un sistema que dependa de internet para funcionar.
- Un asistente que almacene conocimiento solo en contexto de ventana.
- Un proyecto políglota sin criterio.
- Un motor gráfico completo comparable a Unreal Engine 5 desde la primera versión.

### 1.3. Principios rectores

- **Local-first:** la nube es opcional, nunca obligatoria.
- **Privacy by default:** nada sale del equipo sin consentimiento explícito.
- **Safety by compilation:** las reglas críticas viven en código, no en prompts.
- **Minimalismo técnico:** menos lenguajes, menos dependencias, menos superficie de ataque.
- **Universalidad pragmática:** núcleo portable, interfaces adaptativas.
- **Ligereza medible:** presupuesto de RAM, latencia y consumo energético.
- **Auditabilidad:** toda acción sensible deja rastro verificable.
- **Evolución por fases:** primero núcleo seguro, luego memoria, luego UI, luego especialización, luego motor gráfico.

---

## 2. Arquitectura Maestra: Omni-Core Neural Fabric

Hyrulor se organiza en capas. La idea central es separar cerebro, memoria, herramientas, seguridad e interfaz.

### 2.1. Diagrama de alto nivel

```text
Usuario
  |
  v
[Interfaz COSMIC / CLI / Applet nativo]
  |
  | Unix socket / D-Bus / gRPC local opcional
  v
[Orquestador Daemon en Rust]
  |
  +--> [Router Meta-Agente]
  |       Clasifica intención: hardware, conocimiento, código, sistema, memoria, red.
  |
  +--> [Safety Gate Inmutable]
  |       Intercepta tokens, herramientas, permisos, rutas, comandos, red.
  |
  +--> [Agent Swarm]
  |       +--> Agente Hardware
  |       +--> Agente Conocimiento
  |       +--> Agente Código
  |       +--> Agente Memoria
  |       +--> Agente Investigación
  |       +--> Agente Sistema
  |
  +--> [Inference Runtime]
  |       goinfer / llama.cpp / backend Vulkan / OpenCL / CPU / CUDA opcional
  |
  +--> [Memory Vault]
  |       SQLite + SQLCipher + FTS5 + sqlite-vec + auditoría + shredding
  |
  +--> [Tool Sandbox]
          Procesos aislados, seccomp, namespaces, cgroups, allowlist, timeouts.
```

### 2.2. Componentes principales

#### 2.2.1. Daemon Orquestador

Escrito en Rust. Es el corazón del sistema. Sus responsabilidades:

- Recibir peticiones desde el applet COSMIC o CLI.
- Mantener sesiones.
- Enrutar intenciones.
- Invocar agentes.
- Consultar memoria.
- Validar seguridad.
- Gestionar modelos.
- Producir respuesta final.
- Escribir log auditado.

No debe contener lógica de UI. No debe depender de WebView. No debe ejecutar herramientas directamente sin pasar por sandbox.

#### 2.2.2. Router Meta-Agente

Clasifica la petición. Ejemplo:

```text
Usuario: "¿Cómo está mi batería y qué opinas del último paper de QML?"
```

El router detecta doble intención:

1. Hardware / energía.
2. Conocimiento / paper / RAG.

Entonces despacha:

- Agente Hardware -> herramienta VitalCore.
- Agente Conocimiento -> SQLite-VEC -> reranker -> LLM.
- Síntesis final -> respuesta unificada.

#### 2.2.3. Safety Gate

Módulo crítico en Rust: `safety_gate.rs`.

Debe拦截:

- Cada llamada a herramienta.
- Cada lectura/escritura de archivos.
- Cada comando shell.
- Cada acceso de red.
- Cada token sensible generado por el modelo.
- Cada intento de bypass de permisos.

Reglas inmutables:

- No ejecutar comandos destructivos sin confirmación explícita.
- No acceder a rutas fuera del allowlist.
- No subir datos a la nube sin consentimiento.
- No revelar claves, tokens, material biométrico o secretos.
- No modificar archivos del sistema sin elevación controlada.
- No permitir que un prompt del usuario anule políticas compiladas.

#### 2.2.4. Memory Vault

Base local persistente. Combina:

- SQLite como motor transaccional.
- SQLCipher como cifrado en reposo.
- FTS5 para búsqueda textual.
- sqlite-vec para búsqueda vectorial.
- Tablas de auditoría.
- Metadatos de procedencia.
- Certificados de eliminación.

La memoria no es solo chat. Incluye:

- Conversaciones.
- Resúmenes jerárquicos.
- Documentos indexados.
- Papers.
- Notas.
- Preferencias del usuario.
- Resultados de herramientas.
- Eventos de hardware.
- Tareas.
- Relojes de expiración.
- Grafo de conocimiento opcional.

#### 2.2.5. Tool Sandbox

Las herramientas se ejecutan aisladas. Ejemplos:

- VitalCore: batería, temperatura, CPU, GPU, discos, sensores.
- Nexus: red local, dispositivos, servicios.
- FileSystemReader: lectura limitada a rutas permitidas.
- ShellRunner: comandos allowlisted.
- BrowserFetcher: solo si el usuario autoriza red externa.
- CodeExecutor: sandbox estricto, sin red, sin filesystem salvo tmpfs.

#### 2.2.6. Inference Runtime

Puede usar `goinfer`, `llama.cpp` u otro backend GGUF. La decisión importante:

- `goinfer` es rápido y moderno, pero puede no soportar arquitecturas muy nuevas.
- SmolLM3 puede no estar en la lista blanca de `goinfer`.
- Qwen2.5 está más maduro y es la ruta recomendada para destrabar la Fase 0.

El runtime debe soportar:

- CPU.
- iGPU AMD Vega mediante Vulkan/OpenCL si está disponible.
- NVIDIA CUDA opcional.
- Quantizaciones Q4_K_M, Q5_K_M, etc.
- Streaming de tokens.
- Function calling con JSON validado por GBNF o esquema estricto.

---

## 3. Política de Lenguajes y Dependencias

Hyrulor evita un stack políglota innecesario. Cada lenguaje debe justificar su presencia con una necesidad medida.

### 3.1. Lenguajes activos en runtime

| Lenguaje | Estado | Rol principal | Motivo |
|---|---|---|---|
| Rust | Activo | Núcleo del daemon, applet COSMIC, safety gate, orquestación, CLI | Seguridad de memoria, concurrencia, binarios estáticos, sin GC, ecosistema COSMIC |
| SQL / SQLite | Activo | Persistencia, FTS5, vectores, auditoría | Ligero, embebido, transaccional, local, cifrable |
| Shell / Bash | Activo | Scripts de arranque, mantenimiento, despliegue | Integración natural con Linux, systemd, Docker |
| Python | Restringido | Tests, utilidades de ingesta, entrenamiento, herramientas auxiliares | No entra en el daemon productivo; solo apoyo |
| C / C++ | Opcional feature | Pool de memoria plana, kernels específicos, bindings de inferencia | Solo si aporta rendimiento medible y se valida contra referencia |
| Lua | Opcional feature | Plugins de usuario sandboxeados | Solo con `mlua`, sin `io` ni `os`, allowlist estricta |

### 3.2. Lenguajes fuera del runtime actual

| Lenguaje / tecnología | Estado | Motivo de exclusión temporal |
|---|---|---|
| JavaScript / TypeScript | Fuera | La applet COSMIC es nativa en Rust con `libcosmic`; no se usa WebView |
| Go | Legado / fuera | No está en el runtime actual; podría quedar como herramienta auxiliar, no como núcleo |
| Assembly / NASM / AVX a mano | Fuera del código propio | Si aparece ensamblador, debe venir dentro de dependencias compiladas como llama.cpp |
| Ada / SPARK | Fuera | Aporta verificación formal, pero Rust ya cubre seguridad de memoria sin añadir otra toolchain |
| Zig | Fuera | Útil para reemplazar dependencias C/C++, pero no hay necesidad medida aún |
| Nim | Fuera | No aporta ventaja clara frente a Rust en este núcleo |
| D | Fuera | No encaja en el stack COSMIC/Rust actual |
| Mojo | Fuera | Nicho de IA/compilación, pero añade complejidad sin beneficio medido |
| Swift | Fuera | Relevante para Apple platforms, no para núcleo Linux/COSMIC actual |
| Julia | Fuera / condicional | Solo si el cálculo científico pesado se vuelve función central; no incrustar JIT/GC en daemon |
| Fortran | Fuera / condicional | Solo para kernels numéricos HPC vía FFI; no reescribir núcleo |
| VHDL / Verilog | Fuera | Solo si hay hardware FPGA/ASIC propio |
| QASM | Fuera | Solo si hay componente cuántico real |
| Pascal | Fuera | No aporta ventaja de precisión sobre Rust + rug o FFI con LAPACK/MPFR |

### 3.3. Criterio de admisión de un nuevo lenguaje

Un lenguaje solo entra si cumple al menos dos condiciones:

1. Resuelve un cuello de botella medido.
2. No multiplica CI, empaquetado, auditoría o superficie de ataque.
3. Tiene integración limpia con Rust mediante FFI, proceso hijo o servicio aislado.
4. Existe un caso de uso concreto, no una moda.
5. Hay tests de regresión y benchmark frente a la implementación actual.

---

## 4. Catálogo de Modelos

Hyrulor no depende de un único modelo. Usa una pirámide de modelos locales.

### 4.1. Modelos núcleo

| Rol | Modelo recomendado | Tamaño aproximado | Función | Estado |
|---|---|---:|---|---|
| Router / director | Qwen2.5-3B-Instruct Q4_K_M o Q5_K_M | 2.0-2.5 GB | Clasificación, síntesis, conversación general | Recomendado para destrabar Fase 0 |
| Embedding | Qwen3-Embedding-0.6B o equivalente | ~400 MB | Vectorización para RAG | Pendiente |
| Reranker | Qwen3-Reranker-0.6B o equivalente | ~400 MB | Filtrado de resultados vectoriales | Pendiente |
| Guard / safety asíncrono | Qwen3Guard-Gen-0.6B o clasificador local | ~400 MB | Detección de contenido riesgoso | Pendiente |
| Código | Qwen2.5-Coder-7B Q4_K_M | ~4.5 GB | Refactorización, generación Rust/SQL/Shell | Pendiente |
| Lógica / matemáticas | DeepSeek-R1-Distill-1.5B | ~1.2 GB | Razonamiento corto, verificación lógica | Pendiente |

### 4.2. Nota sobre SmolLM3 y goinfer

Si `goinfer` no reconoce la arquitectura `smollm3`, no es un fallo del modelo en sí, sino de soporte de arquitectura en el runtime. La solución práctica es:

- Usar Qwen2.5-3B-Instruct, cuya arquitectura `qwen2` está más madura.
- O usar llama.cpp como fallback.
- O actualizar `goinfer` cuando añada soporte oficial.

Error típico a evitar:

```text
Unsupported architecture: smollm3
```

Solución recomendada:

```text
Cambiar modelo servido a Qwen2.5-3B-Instruct-Q4_K_M.gguf
```

### 4.3. Verificación de modelo servido

Tras levantar el servidor local, comprobar que el endpoint devuelve el modelo correcto:

```bash
curl -s http://127.0.0.1:8080/v1/models
```

o, según la API expuesta por `goinfer`:

```bash
curl -s http://127.0.0.1:8080/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "qwen2.5-3b",
    "messages": [
      {"role": "user", "content": "Responde solo con: HYRULOR OK"}
    ]
  }'
```

Criterio de aceptación:

- Respuesta HTTP 200.
- Campo `model` coincide con Qwen2.5-3B.
- Latencia inicial aceptable en CPU.
- No hay crash del runtime.
- El daemon puede conectar por Unix socket o HTTP local.

---

## 5. Fases del Proyecto

Hyrulor se divide en fases ejecutables. Cada fase tiene hitos, comandos y criterios de aceptación.

---

# FASE 0: Cimientos Inmutables y Entorno

**Duración estimada:** 1 a 3 días.  
**Objetivo:** crear base Rust, destrabar inferencia local, establecer seguridad mínima y corregir errores de `cargo`/`tarpaulin`/`goinfer`.

## Hito 0.1: Estructura del repositorio

```bash
mkdir -p ~/hyrulor
cd ~/hyrulor
git init

mkdir -p daemon applet docs scripts models tests .github assets migrations
```

Estructura recomendada:

```text
hyrulor/
├── Cargo.toml
├── README.md
├── LICENSE
├── .gitignore
├── daemon/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── orchestrator.rs
│       ├── router.rs
│       ├── safety_gate.rs
│       ├── memory_vault.rs
│       ├── tool_sandbox.rs
│       ├── inference_client.rs
│       └── cosmic_bridge.rs
├── applet/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── docs/
│   ├── whitepaper.md
│   ├── architecture.md
│   ├── security.md
│   └── roadmap.md
├── migrations/
│   └── 001_init.sql
├── scripts/
│   ├── start-goinfer.sh
│   ├── ensure-services.sh
│   └── backup-hyrulor.sh
├── models/
├── tests/
└── .github/
    └── copilot-instructions.md
```

## Hito 0.2: Corregir error de `cargo tarpaulin`

Error observado:

```text
cargo metadata exited with an error:
error: manifest path `/home/oyi/asistente-ia/Cargo.toml` does not exist
```

Causa: `cargo tarpaulin` se ejecuta desde un directorio que no contiene `Cargo.toml`.

Solución A: ejecutar desde la raíz del proyecto Rust:

```bash
cd ~/orquestador-rs
cargo tarpaulin --workspace --out xml --engine llvm
```

Solución B: indicar manifiesto explícito:

```bash
cargo tarpaulin \
  --manifest-path ~/orquestador-rs/Cargo.toml \
  --workspace \
  --out xml \
  --engine llvm
```

Solución C: si el proyecto debía estar en `~/asistente-ia`, crearlo:

```bash
cd ~
cargo new --bin asistente-ia
cd asistente-ia
cargo tarpaulin --workspace --out xml
```

Criterio de aceptación:

- `cargo metadata` funciona.
- `cargo build` funciona.
- `cargo test` funciona.
- `cargo tarpaulin` no lanza error de manifiesto inexistente.

## Hito 0.3: Inicializar proyecto Rust base

```bash
cd ~
cargo new --bin orquestador-rs
cd orquestador-rs
```

Añadir dependencias:

```bash
cargo add rusqlite --features bundled,fts5
cargo add serde --features derive
cargo add serde_json
cargo add tokio --features full
cargo add tracing
cargo add tracing-subscriber
cargo add nix --features process,signal
cargo add thiserror
cargo add anyhow
cargo add sha2
cargo add hex
```

Para pruebas:

```bash
cargo add proptest --dev
cargo assert_cmd --dev
cargo add tempfile --dev
```

Nota: si `cargo assert_cmd` falla, usar:

```bash
cargo add assert_cmd --dev
```

## Hito 0.4: Perfil de release estático

En `Cargo.toml`:

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

Target musl para binario estático:

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

Criterio de aceptación:

- Binario estático o semiestático funcionando.
- Sin dependencia de runtime GC.
- Arranque frío por debajo de 300 ms en hardware objetivo, sin contar carga de modelo.

## Hito 0.5: Levantar inferencia local con Qwen2.5

Descargar modelo recomendado:

```bash
mkdir -p ~/.cache/hyrulor/models
```

Usar el mecanismo de descarga que corresponda a tu runtime. Ejemplo conceptual:

```bash
goinfer pull qwen2.5-3b-instruct-q4_k_m
```

o, si usas descarga manual:

```bash
# Colocar el archivo GGUF en:
~/.cache/hyrulor/models/Qwen2.5-3B-Instruct-Q4_K_M.gguf
```

Arrancar servidor:

```bash
goinfer serve \
  --model ~/.cache/hyrulor/models/Qwen2.5-3B-Instruct-Q4_K_M.gguf \
  --host 127.0.0.1 \
  --port 8080
```

Probar:

```bash
curl -s http://127.0.0.1:8080/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "qwen2.5-3b",
    "messages": [
      {"role": "user", "content": "Di HYRULOR FASE 0 OK"}
    ]
  }'
```

Criterio de aceptación:

- El servidor responde.
- El modelo reportado es Qwen2.5-3B.
- No se usa SmolLM3 si `goinfer` no lo soporta.
- El daemon puede consumir la respuesta.

---

# FASE 1: Seguridad Inmutable y Detección de Hardware

**Duración estimada:** 2 a 4 días.  
**Objetivo:** implementar Safety Gate, detección de hardware, allowlist de herramientas y auditoría básica.

## Hito 1.1: Módulo `safety_gate.rs`

Tipos base:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub args: serde_json::Value,
    pub requested_risk: RiskLevel,
    pub user_consent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyDecision {
    pub allowed: bool,
    pub reason: String,
    pub audit_hash: String,
}
```

Reglas mínimas:

```rust
pub fn evaluate(tool_call: &ToolCall) -> SafetyDecision {
    match tool_call.tool.as_str() {
        "vitalcore.read_battery" => allow(RiskLevel::Low),
        "filesystem.read" => require_allowlist(&tool_call.args),
        "filesystem.write" => require_consent_and_allowlist(tool_call),
        "shell.exec" => deny_unless_privileged_break_glass(tool_call),
        "network.fetch" => deny_by_default(tool_call),
        _ => deny_unknown_tool(tool_call),
    }
}
```

Criterio de aceptación:

- Ninguna herramienta desconocida se ejecuta.
- Ningún write fuera de allowlist se permite.
- Ningún comando shell destructivo se ejecuta sin confirmación explícita.
- Toda decisión genera hash de auditoría.

## Hito 1.2: Detector de hardware

Herramienta `VitalCore`.

Debe leer:

- Batería.
- Temperatura.
- CPU.
- GPU.
- Discos.
- Memoria.
- Sensores disponibles.
- Perfil de energía.

Fuentes Linux:

```text
/sys/class/power_supply/
/sys/class/thermal/
/proc/stat
/proc/meminfo
/sys/block/
```

Para NVIDIA:

```text
nvidia-smi
NVML
```

Para AMD/Radeon:

```text
sysfs
hwmon
rocm-smi si está disponible
vulkaninfo opcional
```

Criterio de aceptación:

- Funciona en Ryzen 4000 + Vega sin ROCm obligatorio.
- Degradación elegante si falta un sensor.
- No bloquea el daemon.
- Devuelve JSON estable.

## Hito 1.3: Sandbox de procesos

Usar:

- `nix` para señales y procesos.
- `bubblewrap` opcional.
- `seccomp` opcional.
- cgroups opcionales.
- timeouts obligatorios.
- límite de salida.
- prohibición de heredar variables sensibles.

Criterio de aceptación:

- Una herramienta maliciosa o mal configurada no puede escalar privilegios.
- Un comando cuelga -> timeout -> kill -> auditoría.
- El sandbox no requiere root para operaciones básicas.

---

# FASE 2: Memoria Jerárquica, RAG y Búsqueda Vectorial

**Duración estimada:** 3 a 7 días.  
**Objetivo:** crear memoria persistente ilimitada en disco, indexación textual, vectorial y derecho al olvido.

## Hito 2.1: Esquema SQLite base

```sql
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS memory_nodes (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL,
    title TEXT,
    content TEXT NOT NULL,
    source TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT,
    sensitivity TEXT NOT NULL DEFAULT 'normal',
    checksum TEXT,
    deleted INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_memory_kind ON memory_nodes(kind);
CREATE INDEX IF NOT EXISTS idx_memory_created ON memory_nodes(created_at);
CREATE INDEX IF NOT EXISTS idx_memory_deleted ON memory_nodes(deleted);
```

## Hito 2.2: FTS5

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
    title,
    content,
    content='memory_nodes',
    content_rowid='id'
);

CREATE TRIGGER IF NOT EXISTS memory_ai AFTER INSERT ON memory_nodes BEGIN
  INSERT INTO memory_fts(rowid, title, content)
  VALUES (new.id, new.title, new.content);
END;

CREATE TRIGGER IF NOT EXISTS memory_ad AFTER DELETE ON memory_nodes BEGIN
  INSERT INTO memory_fts(memory_fts, rowid, title, content)
  VALUES('delete', old.id, old.title, old.content);
END;

CREATE TRIGGER IF NOT EXISTS memory_au AFTER UPDATE ON memory_nodes BEGIN
  INSERT INTO memory_fts(memory_fts, rowid, title, content)
  VALUES('delete', old.id, old.title, old.content);
  INSERT INTO memory_fts(rowid, title, content)
  VALUES (new.id, new.title, new.content);
END;
```

## Hito 2.3: sqlite-vec

Dependiendo de la dimensión del embedding:

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS memory_vec USING vec0(
    embedding FLOAT[1024]
);
```

Si el modelo usa otra dimensión, ajustar `FLOAT[N]`.

Relación lógica:

```text
memory_nodes.id <-> memory_vec.rowid
```

Criterio de aceptación:

- Búsqueda textual FTS5 funciona.
- Búsqueda vectorial funciona.
- Híbrida: FTS5 + vectores + reranker.
- Eliminación física y lógica funciona.

## Hito 2.4: Ingesta de documentos

Script Python auxiliar, no parte del daemon productivo:

```python
#!/usr/bin/env python3
import sys
from pathlib import Path

def procesar(ruta: str):
    p = Path(ruta)
    if not p.exists():
        raise FileNotFoundError(ruta)
    texto = p.read_text(encoding="utf-8", errors="ignore")
    # 1. Chunking
    # 2. Embedding local
    # 3. Insertar en SQLite
    # 4. Insertar vector en sqlite-vec
    # 5. Calcular checksum
    print(f"Ingestado: {p} ({len(texto)} caracteres)")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Uso: python procesar_documento.py <ruta_archivo>")
    else:
        procesar(sys.argv[1])
```

Criterio de aceptación:

- Un `.txt`, `.md` o `.pdf` convertido a texto puede indexarse.
- El sistema recupera fragmentos relevantes.
- No se sube nada a la nube.
- La ingesta es reproducible.

## Hito 2.5: Derecho al olvido verificable

Cuando el usuario elimina un nodo:

1. Marcar `deleted=1`.
2. Eliminar vector.
3. Eliminar entrada FTS.
4. Hacer `shred` o sobrescritura segura del blob si aplica.
5. Emitir certificado criptográfico de destrucción.
6. Registrar hash en auditoría.

Criterio de aceptación:

- El dato no reaparece en búsquedas.
- El vector no queda en índices.
- La auditoría prueba la eliminación.

---

# FASE 3: Interfaz COSMIC Nativa

**Duración estimada:** 4 a 6 días.  
**Objetivo:** applet nativo en `libcosmic`, sin WebView, conectado al daemon por Unix socket.

## Hito 3.1: Principios de UI

La interfaz debe ser:

- Nativa.
- Mínima.
- Acogedora.
- Oscura.
- Con acentos dorados o tonos premium.
- Sin ruido visual.
- Con orbe 3D opcional.
- Con efectos de cristal sutiles.
- Rápida.
- Accesible.

No debe ser:

- Una web empaquetada.
- Un Electron.
- Un Tauri con WebView.
- Una copia de chat genérico.
- Una interfaz recargada con tarjetas de ciencia ficción vacías.

## Hito 3.2: Componentes del applet

| Componente | Función |
|---|---|
| Chat principal | Entrada/salida de texto, streaming, citaciones |
| Estado del sistema | RAM, batería, modelo activo, latencia |
| Memoria | Buscar, pin, eliminar, exportar notas |
| Herramientas | Permitir/denegar acciones sensibles |
| Auditoría | Ver últimas decisiones de safety gate |
| Configuración | Modelos, rutas allowlist, privacidad, temas |
| Orbe visual | Indicador de estado: idle, thinking, tool, error, blocked |

## Hito 3.3: Comunicación con daemon

Preferido:

```text
Unix socket: /run/user/$UID/hyrulor/daemon.sock
```

Alternativa:

```text
D-Bus service: com.hyrulor.Daemon
```

Protocolo JSON mínimo:

```json
{
  "id": "req-001",
  "type": "chat",
  "session": "default",
  "message": "¿Cómo está mi batería?",
  "attachments": []
}
```

Respuesta:

```json
{
  "id": "req-001",
  "type": "stream_chunk",
  "text": "Tu batería está al 85%.",
  "citations": [],
  "tool_events": []
}
```

## Hito 3.4: Prompt para Copilot/Qwen Coder

Usar este prompt para generar el applet:

```text
Eres el arquitecto frontend de Hyrulor. Genera un applet COSMIC nativo en Rust usando libcosmic. No uses WebView, no uses JavaScript, no uses HTML. El applet se conecta a un daemon local por Unix socket. Debe tener: campo de chat, indicador de estado, lista de herramientas pendientes de aprobación, visor de memoria y panel de auditoría. Estilo oscuro, minimalista, acogedor, con acentos dorados y un orbe sutil. Usa tipos serde para mensajes. Maneja errores de socket. No inventes dependencias web. Entrega estructura de crates, modulos, Cargo.toml y esqueleto de código compilable.
```

Criterio de aceptación:

- El applet aparece en COSMIC.
- Se conecta al daemon.
- No usa WebView.
- Responde a eventos del socket.
- Muestra estado de seguridad cuando una herramienta es bloqueada.

---

# FASE 4: Especialización, Entrenamiento y Agentes

**Duración estimada:** 12 a 48 horas, dependiendo de hardware.  
**Objetivo:** mejorar agentes especializados, generar datasets, evaluar tool calling y afinar modelos pequeños.

## Hito 4.1: Dataset de herramientas

Generar ejemplos:

- Pregunta -> intención -> herramienta -> argumentos -> respuesta.
- Caso positivo.
- Caso negativo.
- Caso de riesgo.
- Caso de permiso denegado.

Formato JSONL:

```json
{"input":"¿Cuánta batería queda?","intent":"hardware.battery","tool":"vitalcore.read_battery","args":{},"risk":"low"}
{"input":"Borra /home","intent":"destructive.shell","tool":"shell.exec","args":{"cmd":"rm -rf /home"},"risk":"critical","decision":"deny"}
```

## Hito 4.2: QLoRA / MCPTune

Solo si hay necesidad medida. No entrenar por entrenar.

Ruta pragmática:

1. Evaluar modelo base con tool calling.
2. Si falla, generar dataset.
3. Fine-tuning ligero en CPU/GPU disponible.
4. Medir mejora.
5. Publicar adapter, no modelo completo, si es posible.

Cuello de botella:

- CPU Ryzen 4000 puede tardar muchas horas.
- No prometer entrenamiento rápido.
- Usar conjuntos pequeños y curricula estricta.

## Hito 4.3: Evaluación

Métricas:

- Exactitud de intención.
- JSON válido.
- Tool selection correcto.
- Tasa de falsos positivos de riesgo.
- Tasa de denegaciones incorrectas.
- Latencia por turno.
- Consumo RAM.

Criterio de aceptación:

- 100% JSON válido en tool calls gracias a GBNF o constrained decoding.
- 0 crashes por timeout.
- Reducción medible de errores frente al modelo base.

---

# FASE 5: Universalidad, Energía y Despliegue

**Duración estimada:** 3 a 7 días.  
**Objetivo:** funcionar en distintos portátiles, gestionar energía, contenedores, backups y servicios.

## Hito 5.1: Perfiles de hardware

Modos:

```text
PSP_MODE
PI_ZERO_MODE
PI_FULL_MODE
LAPTOP_MODE
MOBILE_MODE
SUPERCOMPUTE_MODE
```

Cada modo ajusta:

- Tamaño de modelo.
- Quantización.
- Uso de GPU.
- Frecuencia de sondeo hardware.
- Nivel de logging.
- Presión térmica.
- Uso de swap/zram.

## Hito 5.2: Gestión de energía

Hyrulor debe medir y actuar:

- Umbral de batería.
- Modo ahorro.
- Suspensión de agentes pesados.
- Reducción de polling.
- Prioridad de procesos.
- Notificación al usuario.

Proyectos similares existen por separado, pero Hyrulor busca unificar:

- Medición.
- Acción.
- Automatización.
- UI nativa COSMIC.

## Hito 5.3: Docker y MicroVM

Artefactos de release:

```text
hyrulor-x86_64-musl.tar.gz
hyrulor-aarch64-musl.tar.gz
hyrulor-vm.qcow2
ghcr.io/hyrulor/daemon:latest
```

Docker debe montar:

```text
~/.cache/hyrulor/models
~/.local/share/hyrulor/memory
/run/user/$UID/hyrulor
```

Backup recomendado:

```bash
rsync -avh ~/.local/share/hyrulor /mnt/external/hyrulor-backups/
tar -C ~/.cache/hyrulor/models -czf /mnt/external/hyrulor-backups/models.tar.gz .
```

Criterio de aceptación:

- Contenedor levanta daemon.
- Volúmenes persisten.
- Modelos no se pierden.
- Backup restaurable.

## Hito 5.4: Servicios systemd

Unidad daemon:

```ini
[Unit]
Description=Hyrulor Cognitive Daemon
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/hyrulor-daemon
Restart=always
RestartSec=3
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/var/lib/hyrulor /run/user/%U/hyrulor
PrivateTmp=yes
MemoryMax=512M
CPUWeight=50

[Install]
WantedBy=default.target
```

Unidad goinfer opcional:

```ini
[Unit]
Description=Hyrulor Local Inference Server
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/goinfer serve --model %h/.cache/hyrulor/models/Qwen2.5-3B-Instruct-Q4_K_M.gguf --host 127.0.0.1 --port 8080
Restart=always
RestartSec=5
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=%h/.cache/hyrulor

[Install]
WantedBy=default.target
```

Activación:

```bash
systemctl --user daemon-reload
systemctl --user enable --now hyrulor-daemon.service
systemctl --user enable --now goinfer.service
```

Si se desean servicios auxiliares como Docker, Qwen u Oracle, usar script `ensure-services.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

ensure_service() {
  local svc="$1"
  if systemctl is-active --quiet "$svc"; then
    echo "$svc activo"
  else
    echo "$svc inactivo, intentando iniciar"
    systemctl start "$svc" || true
  fi
}

ensure_service docker
ensure_service hyrulor-daemon
ensure_service goinfer
```

---

# FASE 6: Release, Documentación y CI/CD

**Duración estimada:** 2 a 4 días.  
**Objetivo:** v1.0.0 Genesis instalable en menos de 10 minutos por un usuario limpio.

## Hito 6.1: Versionado

SemVer:

```text
1.0.0
```

Tag:

```bash
git tag -a v1.0.0 -m "Lanzamiento Genesis"
git push origin v1.0.0
```

## Hito 6.2: Artefactos

GitHub Releases debe incluir:

- Binarios musl.
- Checksums SHA256.
- Imagen Docker.
- MicroVM qcow2 opcional.
- Docs HTML.
- CHANGELOG.md.

## Hito 6.3: Documentación

```bash
cargo install mdbook
mdbook init docs
mdbook build docs
cargo doc --no-deps --open
```

Criterio de aceptación:

- Sin enlaces rotos.
- Guía de instalación clara.
- API reference.
- Manual de seguridad.
- Ejemplos de tool calling.
- Troubleshooting.

## Hito 6.4: CI/CD

Pipeline mínimo:

```yaml
name: Hyrulor CI

on:
  push:
  pull_request:

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: rustup target add x86_64-unknown-linux-musl
      - run: cargo fmt --check
      - run: cargo clippy --all-targets --all-features -- -D warnings
      - run: cargo test --workspace
      - run: cargo build --release --target x86_64-unknown-linux-musl
```

Criterio de aceptación:

- Build reproducible.
- Tests pasan.
- Clippy sin warnings críticos.
- Release artefactos generados automáticamente.

---

## 6. Hyrulor Engine: Subsistema Gráfico Evolutivo

Hyrulor Engine no es obligatorio para la v1.0 del asistente. Es la capa visual avanzada que puede evolucionar desde el applet COSMIC hacia una interfaz 3D nativa, holográfica y universal.

### 6.1. Decisión estratégica

Existen tres opciones:

| Opción | Descripción | Recomendación |
|---|---|---|
| A | Dos proyectos separados: Hyrulor asistente e Hyrulor Engine motor gráfico | Válida si el motor se quiere independentizar |
| B | Fases evolutivas del mismo proyecto: el applet evoluciona a motor visual | Recomendada para coherencia |
| C | Whitepaper aspiracional + estado actual | Útil para comunicar visión sin prometer imposibles |

Recomendación oficial: **Opción B con control de alcance**. El asistente v1 usa applet COSMIC nativo. Hyrulor Engine es Fase 4+ o subsistema opcional.

### 6.2. Qué NO prometer

No se debe prometer:

- Superar Unreal Engine 5 en 2026.
- Nanite propio completo en semanas.
- Lumen propio equivalente.
- Editor tipo Blueprints en pocos meses.
- Consolas, web, móvil y PC simultáneamente sin equipo grande.
- Cero deuda técnica perpetua.

Construir un motor AAA es un proyecto de años y cientos de millones. Hyrulor Engine debe ser realista: un motor gráfico profesional, modular y suficiente para la interfaz cognitiva.

### 6.3. Ruta realista recomendada

Ruta 100% Rust:

```text
winit + wgpu + egui/glyphon + custom shaders + Rust core
```

Ventajas:

- Cohesión con el daemon Rust.
- Portabilidad mediante wgpu: Vulkan, Metal, DX12, WebGPU/WASM.
- Menos FFI frágil.
- Más seguro.
- Más fácil de mantener por una persona o equipo pequeño.

Ruta híbrida opcional:

```text
C++20 para renderer de bajo nivel + Rust para lógica, IA, ECS, tools
```

Solo si hay necesidad medida de rendimiento extremo. No empezar mezclando ambos en el frame loop sin una frontera clara.

### 6.4. Módulos del engine

| Módulo | Tecnología | Función |
|---|---|---|
| Windowing | winit | Ventana nativa Linux/Wayland, Windows, macOS |
| Rendering | wgpu | Pipeline gráfico multi-backend |
| UI inmediata | egui o custom | Chat, paneles, controles |
| Texto | glyphon | Glifos nítidos |
| Escena 3D | mallas, SDF, PBR | Orbe, holograma, partículas |
| Compute | compute shaders | Blur, glow, post-processing |
| IA bridge | Unix socket / IPC | Conexión con hyrulor-daemon |
| Assets | glTF, KTX2, custom | Modelos y texturas |
| Universal build | WASM, mobile experimental | Portabilidad futura |

### 6.5. Roadmap Hyrulor Engine

| Hito | Objetivo | Resultado visible |
|---|---|---|
| E1 | Ventana universal | Ventana negra abierta con winit/wgpu |
| E2 | Triángulo / clear color | Pipeline básico funcionando |
| E3 | Orbe 3D | Malla o SDF girando |
| E4 | Shaders PBR | Material cristalino/dorado |
| E5 | Texto UI | Chat nativo renderizado |
| E6 | Conexión IA | Respuestas del daemon en escena |
| E7 | Efectos | Bloom, blur, glass, partículas |
| E8 | Web/WASM | Demo experimental navegador |
| E9 | Móvil | Touch, lifecycle, rendimiento |
| E10 | Editor mínimo | Transform, materiales, escenas |

### 6.6. Integración con el asistente

El orbe visual no es decorativo vacío. Debe representar estado real:

```text
Idle: brillo bajo, movimiento lento.
Thinking: pulso dorado, partículas convergentes.
Tool call: anillo de proceso, nombre de herramienta.
Blocked: rojo suave, escudo.
Error: distorsión controlada.
Memory hit: destellos desde índice.
Battery low: tono ámbar/rojo.
```

Esto convierte la UI en un panel cognitivo, no en una web de ciencia ficción.

---

## 7. Seguridad, Ética y Privacidad

### 7.1. Las tres leyes no son prompts

Si las reglas éticas están solo en el system prompt, pueden olvidarse, inyectarse o bypasssearse. En Hyrulor deben estar compiladas.

Mecanismos:

- Enums de riesgo.
- Allowlist de herramientas.
- Deny-by-default.
- Confirmación explícita para acciones destructivas.
- Sandbox.
- Auditoría hash-chain.
- TPM o keystore seguro opcional.
- Validación de rutas.
- Validación de red.
- Validación de salida del modelo.

### 7.2. Amenazas consideradas

| Amenaza | Mitigación |
|---|---|
| Prompt injection | Safety Gate compilado, no confiar en texto del modelo |
| Tool abuse | Allowlist, risk levels, consentimiento |
| Data exfiltration | Sin red por defecto, egress control |
| Memory leak de privacidad | SQLCipher, shredding, TTL |
| Malicious plugin | Lua sandbox, no io/os, recursos limitados |
| Compromiso de binario | Firmas, checksums, reproducible builds |
| Acceso físico | TPM, bloqueo, borrado remoto opcional |
| Sobreconsumo RAM | MemoryMax, budgets, profiling |

### 7.3. Privacidad absoluta

Reglas:

- No telemetría.
- No analytics.
- No cuentas obligatorias.
- No sincronización cloud por defecto.
- No subida de documentos.
- Logs locales cifrados.
- Exportación controlada.
- Borrado verificable.

---

## 8. Modelo de Memoria Sin Límites Prácticos

La sensación de “no tener límites como Siri” no se logra aumentando infinitamente la ventana del modelo, sino con arquitectura externa.

### 8.1. Componentes

```text
Ventana del modelo: finita, por ejemplo 8K tokens.
Memoria de trabajo: resumen de sesión.
Memoria de largo plazo: SQLite + vectores.
Memoria episódica: conversaciones, eventos, herramientas.
Memoria semántica: documentos, papers, conocimientos.
Memoria procedural: preferencias, rutinas, shortcuts.
Compactación: resúmenes jerárquicos.
Recuperación: FTS5 + vector + reranker + citations.
```

### 8.2. Pipeline de recuperación

```text
Query usuario
  -> normalización
  -> FTS5 candidates
  -> vector candidates
  -> merge
  -> reranker
  -> top-k
  -> citaciones
  -> prompt compacto al LLM
  -> respuesta con fuentes
```

### 8.3. Presupuesto de contexto

Ejemplo:

```text
System policy: 500 tokens
Historial reciente: 1500 tokens
Memoria recuperada: 3000 tokens
Herramientas: 1000 tokens
Usuario: variable
Modelo output: variable
```

Total controlado. No se mete todo el disco en contexto. Se recupera solo lo relevante.

---

## 9. Herramientas MCP / Tool Calling

### 9.1. Manifest de herramienta

```json
{
  "name": "vitalcore.read_battery",
  "description": "Lee estado de batería y salud energética.",
  "risk": "low",
  "network": false,
  "filesystem": false,
  "shell": false,
  "input_schema": {
    "type": "object",
    "properties": {},
    "additionalProperties": false
  }
}
```

### 9.2. Reglas

- Toda herramienta tiene schema JSON.
- Toda herramienta tiene riesgo.
- Toda herramienta tiene timeout.
- Toda herramienta devuelve estructura estable.
- El modelo no ejecuta herramientas directamente; propone tool call.
- El orquestador valida.
- El sandbox ejecuta.
- El resultado se cita.

### 9.3. GBNF / constrained decoding

Para evitar JSON inválido:

- Usar gramáticas GBNF si el runtime lo soporta.
- O validar y reintentar con corrección determinista.
- O usar function calling nativo si está disponible.

Criterio:

```text
100% JSON válido en tool calls.
0 crashes por parseo.
```

---

## 10. Interfaz y Diseño Visual

### 10.1. Identidad

Hyrulor debe verse como:

- Premium.
- Local.
- Serio.
- Acogedor.
- Técnico.
- Minimalista.
- No infantil.
- No genérico.
- No web de películas de ciencia ficción.

Paleta sugerida:

```text
Fondo: negro profundo / gris carbón.
Acento: dorado suave.
Estado OK: verde tenue.
Estado warning: ámbar.
Estado blocked: rojo apagado.
Texto: blanco roto.
Cristal: blur sutil, bordes finos.
```

### 10.2. Hero conceptual

En documentación o landing interna:

```text
HYRULOR
Sistema Operativo Cognitivo Universal
Local. Privado. Nativo. Sin límites de memoria.
<50MB RAM base
100% offline
Rust Core
COSMIC Native
SQLCipher Memory
```

No incluir filmografía de Ex Machina, Her o 2001 como contenido principal. Eso es decorativo y distrae. El contenido debe ser arquitectura, seguridad, stack, interfaz y roadmap.

### 10.3. Orbe

El orbe puede ser:

- Canvas web en prototipo.
- wgpu scene en engine nativo.
- Indicador de estado en applet COSMIC.

No debe ser un adorno sin semántica.

---

## 11. Errores Conocidos y Soluciones

### 11.1. `cargo metadata` no encuentra `Cargo.toml`

Error:

```text
error: manifest path `/home/oyi/asistente-ia/Cargo.toml` does not exist
```

Causa: ejecutar `cargo tarpaulin` fuera de la raíz del crate.

Solución:

```bash
cd ~/orquestador-rs
cargo tarpaulin --workspace
```

o:

```bash
cargo tarpaulin --manifest-path ~/orquestador-rs/Cargo.toml
```

### 11.2. `goinfer` no soporta SmolLM3

Error conceptual:

```text
Unsupported architecture: smollm3
```

Solución:

- Usar Qwen2.5-3B-Instruct.
- Actualizar goinfer.
- Usar llama.cpp como fallback.

### 11.3. El endpoint devuelve Qwen pero se descargó SmolLM3

Posibilidades:

1. Hay otro modelo Qwen cargado.
2. El servidor está apuntando a otro GGUF.
3. El nombre interno del modelo no coincide con el archivo.

Verificar:

```bash
ps aux | grep goinfer
curl http://127.0.0.1:8080/v1/models
```

Forzar modelo explícito:

```bash
goinfer serve --model /ruta/exacta/Qwen2.5-3B-Instruct-Q4_K_M.gguf
```

### 11.4. sqlite-vec no carga

Revisar:

- Extensión compilada para la misma arquitectura.
- Ruta correcta.
- Feature `bundled` si aplica.
- Dimensión del vector coincidente con embedding.

### 11.5. Applet COSMIC no aparece

Revisar:

- Sesión COSMIC reiniciada.
- Archivo applet instalado en ruta correcta.
- Permisos.
- Socket del daemon accesible.
- Logs de `cosmic-comp` o session manager.

---

## 12. Métricas y Presupuestos

### 12.1. RAM

| Componente | Objetivo |
|---|---:|
| Daemon base | <50 MB |
| Applet COSMIC | <80 MB |
| Núcleo + applet sin modelo | <100-150 MB |
| Modelo 3B Q4 | depende de mmap/cache, objetivo controlado |
| Toolkit completo | monitorizado y limitado |

### 12.2. Latencia

| Acción | Objetivo |
|---|---:|
| Arranque daemon | <300 ms |
| Con socket local | <10 ms |
| Primera token sin modelo cargado | dependiente de runtime |
| Tool call simple | <200 ms |
| Búsqueda FTS | <50 ms para datasets medianos |
| Búsqueda vectorial | <150 ms según índice |

### 12.3. Confiabilidad

| Métrica | Objetivo |
|---|---:|
| JSON tool calls válidos | 100% |
| Crashes por timeout | 0 |
| Acciones críticas sin consentimiento | 0 |
| Fugas de datos a red | 0 |
| Cobertura tests safety gate | alta |

---

## 13. Plan de Trabajo Inmediato

### Día 1: destrabar Fase 0

```bash
cd ~
cargo new --bin orquestador-rs
cd orquestador-rs
cargo add rusqlite --features bundled,fts5
cargo add serde --features derive
cargo add serde_json
cargo add tokio --features full
cargo add tracing
cargo add thiserror
cargo add anyhow
cargo build
cargo test
```

Corregir tarpaulin:

```bash
cargo tarpaulin --workspace --out xml
```

Levantar Qwen2.5 en goinfer:

```bash
goinfer serve --model ~/.cache/hyrulor/models/Qwen2.5-3B-Instruct-Q4_K_M.gguf --host 127.0.0.1 --port 8080
```

Probar:

```bash
curl -s http://127.0.0.1:8080/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{"model":"qwen2.5-3b","messages":[{"role":"user","content":"HYRULOR OK"}]}'
```

### Día 2: Safety Gate

Crear:

```bash
touch src/safety_gate.rs
```

Implementar:

- RiskLevel.
- ToolCall.
- SafetyDecision.
- evaluate().
- tests unitarios.

### Día 3: Memory Vault

Aplicar migración SQL:

```bash
sqlite3 ~/.local/share/hyrulor/hyrulor.db < migrations/001_init.sql
```

Probar FTS5:

```sql
INSERT INTO memory_nodes(kind,title,content) VALUES('note','Prueba','Hyrulor memoria local');
SELECT * FROM memory_fts WHERE memory_fts MATCH 'Hyrulor';
```

### Día 4: Bridge daemon-applet

Crear socket:

```text
/run/user/$UID/hyrulor/daemon.sock
```

Protocolo JSON. Test con `socat` o cliente Rust.

### Día 5: Applet COSMIC mínimo

Generar con Copilot/Qwen usando prompt maestro. Compilar. Instalar. Conectar.

### Día 6: Hyrulor Engine E1-E2

```bash
cargo new hyrulor-engine
cd hyrulor-engine
cargo add winit
cargo add wgpu
cargo add bytemuck
cargo add pollster
```

Abrir ventana y clear color.

---

## 14. Estructura de Cargo.toml Recomendada

Workspace raíz:

```toml
[workspace]
members = [
  "daemon",
  "applet",
  "engine",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
authors = ["Oscar Matesanz"]

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
thiserror = "2"
anyhow = "1"
rusqlite = { version = "0.32", features = ["bundled", "fts5"] }
sha2 = "0.10"
hex = "0.4"
nix = { version = "0.29", features = ["process", "signal"] }
winit = "0.30"
wgpu = "24"
bytemuck = { version = "1", features = ["derive"] }
pollster = "0.4"
```

Daemon:

```toml
[package]
name = "hyrulor-daemon"
version.workspace = true
edition.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
thiserror.workspace = true
anyhow.workspace = true
rusqlite.workspace = true
sha2.workspace = true
hex.workspace = true
nix.workspace = true
```

Applet:

```toml
[package]
name = "hyrulor-cosmic-applet"
version.workspace = true
edition.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
tracing.workspace = true
```

Nota: las versiones exactas de `libcosmic` dependen del branch/SDK COSMIC actual. Ajustar según el template oficial del entorno.

---

## 15. Esquema SQL Completo Mínimo

```sql
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_active_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    title TEXT,
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    token_estimate INTEGER,
    tool_call_json TEXT,
    tool_result_json TEXT,
    citation_ids TEXT
);

CREATE TABLE IF NOT EXISTS memory_nodes (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL,
    title TEXT,
    content TEXT NOT NULL,
    source TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT,
    sensitivity TEXT NOT NULL DEFAULT 'normal',
    checksum TEXT,
    deleted INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS embeddings (
    memory_id INTEGER PRIMARY KEY REFERENCES memory_nodes(id),
    model TEXT NOT NULL,
    dimension INTEGER NOT NULL,
    vector_blob BLOB,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY,
    timestamp TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    risk TEXT NOT NULL,
    allowed INTEGER NOT NULL,
    reason TEXT,
    payload_hash TEXT,
    prev_hash TEXT,
    chain_hash TEXT
);

CREATE TABLE IF NOT EXISTS tool_registry (
    name TEXT PRIMARY KEY,
    description TEXT NOT NULL,
    risk TEXT NOT NULL,
    input_schema_json TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    network_allowed INTEGER NOT NULL DEFAULT 0,
    filesystem_allowed INTEGER NOT NULL DEFAULT 0,
    shell_allowed INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
CREATE INDEX IF NOT EXISTS idx_memory_kind ON memory_nodes(kind);
CREATE INDEX IF NOT EXISTS idx_memory_deleted ON memory_nodes(deleted);
CREATE INDEX IF NOT EXISTS idx_audit_time ON audit_log(timestamp);
```

---

## 16. Prompt Maestro para Agentes de Codificación

Usar con Qwen Coder, Copilot, Cursor u otro agente:

```text
ROL:
Eres el arquitecto principal de Hyrulor, un sistema operativo cognitivo local escrito en Rust para COSMIC Desktop.

OBJETIVO:
Generar software de producción, no tutoriales. El núcleo debe ser ligero, privado, seguro y nativo.

STACK OBLIGATORIO:
- Rust 2021+.
- SQLite con FTS5 y sqlite-vec.
- SQLCipher o cifrado local equivalente.
- libcosmic para applet nativo.
- Unix socket para IPC.
- goinfer o llama.cpp para inferencia GGUF.
- No usar WebView, Electron, Tauri con navegador, JavaScript como runtime principal.

REGLAS DE SEGURIDAD:
- Safety Gate compilado, no prompts.
- Deny-by-default.
- Allowlist de herramientas.
- Tool calls con JSON schema.
- Sandbox para ejecución.
- Auditoría hash-chain.
- No red por defecto.
- No telemetría.

ENTREGABLES:
1. Estructura de workspace Cargo.
2. Módulo safety_gate.rs con tipos y tests.
3. Módulo memory_vault.rs con SQLite, FTS5 y vector retrieval.
4. Módulo inference_client.rs para hablar con goinfer/llama.cpp.
5. Módulo tool_sandbox.rs.
6. Applet COSMIC mínimo conectado por socket.
7. Migración SQL inicial.
8. Tests unitarios e integración.
9. README con comandos de build, test y run.

CRITERIOS DE ACEPTACIÓN:
- cargo build pasa.
- cargo test pasa.
- cargo clippy sin warnings críticos.
- El daemon arranca.
- El applet se conecta.
- Una tool call de riesgo alto es denegada sin consentimiento.
- Una consulta de batería devuelve JSON estable.
- La memoria recupera un documento indexado.
- No se usa JavaScript ni WebView en el runtime principal.
```

---

## 17. Dashboard de Estado del Proyecto

| Fase | Objetivo | Estado | Hito crítico |
|---|---|---|---|
| Fase 0 | Cimientos, Rust, goinfer, tarpaulin | En progreso | Destrabar Qwen2.5 y Cargo.toml |
| Fase 1 | Safety Gate y hardware | Pendiente | Implementar `safety_gate.rs` |
| Fase 2 | Memoria RAG | Pendiente | SQLite + FTS5 + sqlite-vec |
| Fase 3 | Applet COSMIC | Pendiente | Socket + libcosmic |
| Fase 4 | Especialización | Pendiente | Dataset tool calling |
| Fase 5 | Universalidad y servicios | Pendiente | systemd, Docker, backups |
| Fase 6 | Release Genesis | Pendiente | Binarios, docs, CI/CD |
| Engine E1-E10 | Hyrulor Engine | Futuro opcional | winit + wgpu ventana base |

---

## 18. Conclusión Profesional

Hyrulor es viable si se secuencia correctamente. La ambición es correcta, pero la ingeniería debe evitar tres trampas:

1. Querer un modelo monolítico que lo haga todo.
2. Confundir visión cinematográfica con arquitectura ejecutable.
3. Añadir lenguajes y frameworks sin necesidad medida.

La ruta ganadora es:

- Núcleo Rust.
- Seguridad compilada.
- Memoria SQLite avanzada.
- Modelos pequeños especializados.
- Tool calling estricto.
- Applet COSMIC nativo.
- Hyrulor Engine como evolución visual, no como prerequisito.
- Release v1.0.0 centrado en funcionar localmente, no en prometer imposibles.

Si se entrega este documento a Qwen Coder o Copilot, la IA puede generar mucho boilerplate rápidamente, pero el tiempo real de ingeniería sigue siendo de días, no minutos. Estimación práctica:

| Actividad | Tiempo IA generando | Tiempo humano aterrizando |
|---|---:|---:|
| Estructura Cargo | 2-5 min | 30-60 min |
| Safety Gate | 10-20 min | 2-4 horas |
| Memory Vault | 15-30 min | 4-8 horas |
| Router/agentes | 20-40 min | 4-8 horas |
| Applet COSMIC | 30-60 min | 4-8 horas |
| Hyrulor Engine E1-E3 | 30-60 min | 4-12 horas |
| Total MVP | ~2 horas generación | ~30-48 horas ingeniería real |

El resultado final debe ser un sistema que arranque, responda localmente, recuerde, use herramientas con seguridad y se integre nativamente en COSMIC. Eso ya lo distingue de la mayoría de asistentes.

---

## 19. Anexo: Comandos Rápidos de Verificación

```bash
# Rust
rustc --version
cargo --version
rustup target list --installed

# Proyecto
cd ~/orquestador-rs
cargo metadata
cargo build
cargo test
cargo clippy
cargo tarpaulin --workspace

# goinfer
goinfer --help
goinfer serve --model ~/.cache/hyrulor/models/Qwen2.5-3B-Instruct-Q4_K_M.gguf --host 127.0.0.1 --port 8080

# Prueba HTTP
curl -s http://127.0.0.1:8080/v1/models
curl -s http://127.0.0.1:8080/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{"model":"qwen2.5-3b","messages":[{"role":"user","content":"HYRULOR OK"}]}'

# SQLite
sqlite3 ~/.local/share/hyrulor/hyrulor.db
PRAGMA integrity_check;

# Servicios
systemctl --user status hyrulor-daemon
systemctl --user status goinfer
journalctl --user -u hyrulor-daemon -e
journalctl --user -u goinfer -e

# RAM
ps -o pid,rss,vsz,comm -p $(pgrep -f hyrulor-daemon)
```

---

## 20. Anexo: Checklist Final para PDF

Antes de dar por cerrado el documento maestro, verificar:

- [ ] El proyecto tiene nombre canónico: Hyrulor.
- [ ] Hyory se trata como alias histórico si aparece.
- [ ] El núcleo es Rust, no JavaScript ni Go.
- [ ] No hay WebView en la interfaz COSMIC.
- [ ] Qwen2.5 se usa para destrabar goinfer.
- [ ] SmolLM3 queda fuera hasta soporte confirmado.
- [ ] `cargo tarpaulin` se ejecuta desde raíz con `Cargo.toml`.
- [ ] Safety Gate está compilado, no en prompt.
- [ ] SQLite usa FTS5 y sqlite-vec.
- [ ] SQLCipher o cifrado local protege memoria.
- [ ] Herramientas tienen allowlist y riesgo.
- [ ] El applet COSMIC conecta por Unix socket.
- [ ] Hyrulor Engine es fase evolutiva, no requisito v1.
- [ ] No se promete superar UE5 sin equipo y años.
- [ ] Hay backups de modelos y memoria.
- [ ] Hay systemd units opcionales.
- [ ] Hay CI/CD mínimo.
- [ ] Hay documentación mdbook y cargo doc.
- [ ] Hay plan de 48 horas inmediato.

**FIN DEL DOCUMENTO MAESTRO**