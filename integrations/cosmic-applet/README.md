# Hyrulor COSMIC Applet — Fase 3

Applet nativa Rust/libcosmic con popup de chat, tema verde bosque/latón y transporte JSON por líneas hacia el daemon local.

## Estado de esta entrega

**Incluido:**
- Proceso de applet nativo para COSMIC (`cosmic::applet::run`).
- Botón de panel con icono simbólico, popup fijo de 420 × 620, historial mientras el proceso sigue abierto.
- Chat multi-turno, entrada por Enter o botón, acciones rápidas de batería/red/sistema y renderizado incremental de mensajes.
- Socket Unix configurable mediante `HYORY_SOCKET`; por compatibilidad también
  se acepta `HYRULOR_SOCKET`. El valor predeterminado es `/tmp/omni.sock`.
- Mensaje JSON `{"request":"chat","query":"..."}\n` y recepción de JSON-lines.
- Reintentos de conexión con backoff exponencial (250 ms, doblando, límite de 30 s; siete intentos).
- Validación previa del socket: debe ser un socket Unix real, propiedad del usuario actual y con permisos sin acceso de grupo/otros (normalmente `0600`).
- Límites defensivos del protocolo: 1 MiB por frame JSON y 4 MiB por respuesta completa.
- Errores visibles y botón de reintento.
- Binario `hyrulor-mock-daemon` para probar la interfaz sin conectar todavía el daemon real.
- Instalador local de usuario y entrada `.desktop` COSMIC.

**Todavía no incluido:**
- No se puede afirmar que compile en este entorno: aquí no hay toolchain Rust ni COSMIC Desktop para ejecutar `cargo build` o probar la integración real.
- No se han generado screenshots de panel/popup/error porque no hay un compositor COSMIC disponible en este entorno.
- Los efectos 3D/4D completos (WGSL, partículas, aurora, parallax, anillo rotatorio, shimmer y animación palabra a palabra) no están integrados en esta primera entrega.
- Las fuentes no se han copiado al proyecto. No se descargan en runtime; la versión actual usa las fuentes disponibles en el sistema. Consulte `fonts/README.md`.
- No se garantiza binario <2 MB, RAM <15/35 MB, CPU 0% ni cold start <100 ms. Son objetivos que deben medirse en la máquina real; el stack nativo COSMIC tiene dependencias sustanciales.

## Requisitos

- COSMIC Desktop y su panel (la applet no se integra en el panel de GNOME).
- Rust/Cargo 1.93 o posterior, requerido por la versión actual de libcosmic.
- Dependencias de compilación habituales de libcosmic en Ubuntu:

```sh
sudo apt update
sudo apt install -y build-essential cmake pkg-config libexpat1-dev \
  libfontconfig-dev libfreetype-dev libxkbcommon-dev libwayland-dev
```

## Compilar

```sh
cd hyrulor-cosmic-applet
cargo build --release --bin hyrulor-cosmic-applet
```

Para revisar formato, ejecutar las pruebas unitarias y consultar el tamaño/enlace del binario:

```sh
./verify.sh
```

Las versiones directas de `futures`, `libc`, `serde`, `serde_json` y `tokio` están fijadas en `Cargo.toml`. `libcosmic` se consume desde el repositorio oficial Git; la primera resolución generará `Cargo.lock`, que fija también el commit exacto de `libcosmic` y las dependencias transitivas. Genere ese archivo con `cargo generate-lockfile` o durante el primer build y consérvelo en el control de versiones. No se incluye un `Cargo.lock` inventado: debe producirse con Cargo en el equipo de compilación.

## Probar sin el daemon real

Terminal 1:

```sh
cargo run --release --bin hyrulor-mock-daemon
```

Terminal 2:

```sh
cargo run --release --bin hyrulor-cosmic-applet
```

El daemon de prueba crea `/tmp/omni.sock` con permisos `0600` y devuelve respuestas en varios frames JSON para ejercitar el streaming.

Si el daemon real utiliza otra ruta:

```sh
HYORY_SOCKET="$XDG_RUNTIME_DIR/hyrulor/omni.sock" \
  cargo run --release --bin hyrulor-cosmic-applet
```

El daemon real debe crear el socket como usuario del escritorio, con permisos `0600`. Aunque se conserva `/tmp/omni.sock` como ruta por compatibilidad con la especificación, para un despliegue endurecido es preferible un directorio privado bajo `$XDG_RUNTIME_DIR`.

## Instalar en el usuario actual

```sh
./install.sh
```

Después, cierre sesión o reinicie el panel COSMIC y añada Hyrulor desde la configuración de applets del panel.

## Protocolo del daemon

Petición:

```json
{"request":"chat","query":"¿Cuánta batería tengo?"}
```

Respuesta final compatible con el protocolo original:

```json
{"status":"ok","data":{"answer":"Su batería está al 85%."}}
```

Frames opcionales de streaming:

```json
{"status":"tool","data":{"tool":"Consultando sensores…"}}
{"status":"streaming","data":{"delta":"Su batería "}}
{"status":"streaming","data":{"delta":"está al 85%."}}
{"status":"ok","data":{"answer":"Su batería está al 85%."}}
```

Error:

```json
{"status":"error","message":"No se pudo consultar el sensor."}
```

Cada objeto debe terminar en `\n`. En `status=ok`, `data.answer` representa el texto final completo.

## Seguridad y límites

- El cliente no interpreta texto del modelo como comandos. Solo transmite el mensaje `chat` al daemon.
- Esta applet no implementa el Safety Gate, sandbox, TPM ni autorización de herramientas: esas funciones pertenecen al daemon Hyrulor en `src/foundation/` y `src/tool_sandbox.rs`.
- La comprobación de socket reduce el riesgo de conectarse a un endpoint suplantado, pero no reemplaza autenticación de protocolo, autorización por acción ni aislamiento del daemon.
- No configure el socket con permisos `0666`.

## Estructura

```text
src/
├── main.rs
├── app.rs
├── socket.rs
├── bin/
│   └── mock_daemon.rs
└── ui/
    ├── mod.rs
    └── theme.rs
resources/
└── com.github.oyimatesanz-glitch.hyrulor-applet.desktop
THEME.md
LICENSE
install.sh
verify.sh
```
