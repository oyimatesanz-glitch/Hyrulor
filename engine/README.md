# 🜁 Hyrulor Engine

Motor gráfico nativo de **Hyrulor** (100% Rust): un **orbe holográfico** cuyo
color, pulso y brillo codifican el estado real del sistema, junto a una
interfaz de chat nativa que habla con el daemon `hyory` por **socket Unix**
(JSON-Lines). Sin WebView, sin Electron, sin JavaScript.

```
winit + wgpu + egui + custom WGSL shaders + Rust core
```

## Estado

| Hito | Objetivo | Estado |
|---|---|---|
| E1 | Ventana universal (winit) | ✅ |
| E2 | Pipeline básico (wgpu) | ✅ |
| E3 | Orbe 3D (esfera + anillo orbital) | ✅ |
| E4 | Shader holográfico (Fresnel + escaneo + pulso) | ✅ |
| E5 | Texto / chat nativo (egui) | ✅ |
| E6 | Conexión con el daemon (socket Unix) | ✅ |
| E6+ | Autorización humana en la ventana | ✅ |
| E7 | Bloom / blur (cristal) | ✅ (partículas pendientes) |
| E8 | Web/WASM | Pendiente |
| E9 | Móvil | Pendiente |
| E10 | Editor mínimo | Pendiente |

## Requisitos

* Rust 2021+ (`rustc`/`cargo`).
* `wgpu` funciona sobre Vulkan (Linux/AMD Vega), DX12 o Metal según la plataforma.

## Compilar y ejecutar

```bash
cd hyrulor-engine
cargo build --release
./run.sh          # arranca el motor con watchdog (reinicio automático)
./run.sh stop     # lo detiene
./run.sh status   # estado de watchdog y motor
```

### Nota sobre Wayland (SIGSEGV del clipboard)

El motor usa `winit` + `wgpu`. En la sesión **COSMIC** (Wayland), la
integración de clipboard de `winit`/`smithay-clipboard` sufre una **race en
`libwayland-client`** que provoca un `SIGSEGV` intermitente fuera de nuestro
código. Por eso `run.sh` lanza el motor por **X11/XWayland** (sin ese hilo de
clipboard); la ventana sigue siendo una ventana normal del compositor. Para
forzarlo a mano:

```bash
env -u WAYLAND_DISPLAY XDG_SESSION_TYPE=x11 ./target/release/hyrulor-engine
```

## Conexión con el daemon

El motor resuelve el socket del daemon en este orden:

1. `HYRULOR_DAEMON_SOCKET`
2. `HYORY_SOCKET`
3. `/tmp/omni.sock` (por defecto)

Al arrancar comprueba salud y modo del daemon:

```bash
# con el daemon en marcha
./hyrulor.sh start          # en el repo hyrulor/
cargo run --release         # en hyrulor-engine/

# socket explícito
HYRULOR_DAEMON_SOCKET=/tmp/omni.sock cargo run --release
```

Si el daemon no responde, el orbe queda en **modo sin conexión** (gris) y el
chat muestra el error sin bloquear la ventana. El motor **resondea** el estado
cada 5 s (con timeout de 3 s por sonda), así que detecta automáticamente que el
daemon arranca o se cae sin reiniciar la ventana.

## El orbe no es decorativo

| Estado | Color | Significado |
|---|---|---|
| En línea | Oro suave | Sin petición en curso |
| Pensando… | Oro brillante, pulso rápido | Petición en vuelo |
| Autorización requerida | Rojo suave | El daemon espera decisión humana |
| Error | Rojo apagado | Fallo del daemon o del protocolo |
| Sin conexión | Gris | Daemon inaccesible |

## Autorización humana (human-in-the-loop)

Cuando el daemon devuelve `requires_auth` (acción ofensiva), la ventana muestra
un formulario nativo:

* **Modo simple:** pregunta «¿Seguro que quieres hacer esto?» con
  *Autorizar* / *Cancelar*.
* **Modo seguro:** formulario completo (`target`, `scope`, `justification`,
  `start_time`, `end_time`, `authorization_code`) con el código de autorización
  generado por el daemon.

La decisión se envía de vuelta por el mismo socket; el daemon valida, ejecuta
aislado en `bubblewrap` y audita cada paso. El motor solo presenta la decisión,
nunca ejecuta nada por su cuenta.

## Arquitectura

* `src/main.rs` — bucle de render (`winit` + `wgpu`) e interfaz (`egui`).
* `src/orb.rs` — estado del orbe (`OrbState`) y geometría procedural.
* `src/daemon.rs` — cliente JSON-Lines del daemon (con tests de protocolo).
* `shaders/orb.wgsl` — Fresnel, escaneo holográfico y pulso por estado.
* `shaders/post.wgsl` — bloom por umbral de luminancia y composite (cristal).

Las peticiones al daemon se ejecutan en hilos y se devuelven al bucle de
render por un canal `mpsc`, de modo que la interfaz nunca se bloquea.

## Solución de problemas

* Si la ventana no aparece, comprueba que hay sesión Wayland/X11 y un backend
  Vulkan/GL operativo (`RUST_LOG=warn cargo run --release`).
* Los shaders WGSL se validan al crear el dispositivo; un error de sintaxis o
  de layout del uniforme aparece como `Shader validation error` en el log.
* Un daemon lento no congela la UI: las peticiones van en hilos aparte.

## Tests

```bash
cargo test          # tests del cliente del daemon (protocolo JSON-Lines)
cargo clippy --all-targets
```
