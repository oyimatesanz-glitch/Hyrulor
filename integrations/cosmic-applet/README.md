# Hyrulor COSMIC Applet — Fase 3

Applet nativa Rust/libcosmic con popup de chat, tema verde bosque/latón y transporte JSON por líneas hacia el daemon local.

## Estado de esta entrega

**Incluido:**
- Proceso de applet nativo para COSMIC (`cosmic::applet::run`).
- Botón de panel con icono simbólico, popup fijo de 420 × 620, historial mientras el proceso sigue abierto.
- Chat multi-turno, entrada por Enter o botón, acciones rápidas de batería/red/sistema y renderizado incremental de mensajes.
- **Autorización humana integrada:** formulario completo en modo seguro (`target`, `scope`, `justification`, `start_time`, `end_time`, `authorization_code`) y confirmación `SÍ/NO` en modo simple. La applet nunca ejecuta la acción: solo envía la decisión al daemon, que valida el código, aplica el sandbox y audita.
- **Cambio de modo** desde la cabecera (`🔐 SEGURO` / `🔓 SIMPLE`).
- **Comandos de sentidos:** `/imagen <ruta> [pregunta]`, `/voz <archivo.wav>`, `/escuchar [segundos]` y `/hablar <texto>`.
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
- No se han generado screenshots de panel/popup/error porque no hay un compositor COSMIC disponible en este entorno. La compilación y las pruebas unitarias sí se ejecutan con el toolchain local.
- El formulario cubre el ciclo completo de autorización, pero la selección de archivos para visión/voz se hace escribiendo la ruta (no hay diálogo del portal todavía).
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

Autorización requerida (modo simple y modo seguro):

```json
{"status":"requires_auth","mode":"secure","request_id":"AUTH-123","form":{"question":"…","authorization_code":"123456","fields":[…],"proposed_action":{…}}}
```

La applet responde con una nueva conexión:

```json
{"request":"authorize","id":"AUTH-123","approved":true,"form":{"target":"…","scope":"…","justification":"…","start_time":"…","end_time":"…","authorization_code":"123456"}}
```

Modo y sentidos:

```json
{"request":"get_mode"}
{"request":"set_mode","secure":true}
{"request":"describe_image","path":"/ruta/foto.png","prompt":"Describe la imagen"}
{"request":"transcribe","path":"/ruta/audio.wav"}
{"request":"listen","seconds":5}
{"request":"speak","text":"Hola"}
```

Error:

```json
{"status":"error","message":"No se pudo consultar el sensor."}
```

Cada objeto debe terminar en `\n`. En `status=ok`, `data.answer` representa el texto final completo. `data.text` devuelve una transcripción y `data.played`/`data.audio` el resultado de Piper.

## Seguridad y límites

- El cliente no interpreta texto del modelo como comandos. Los comandos `/imagen`, `/voz`, `/escuchar` y `/hablar` son locales y se traducen a peticiones explícitas del protocolo.
- La applet **presenta** la autorización, pero no la concede por sí sola: el código, la validez temporal y la ejecución sandboxed se comprueban en el daemon (`src/protocol.rs`, `src/tool_sandbox.rs`).
- El Guardián (`Qwen3Guard`), el Reranker y la auditoría cifrada con GPG AES-256 viven en el daemon. La applet solo muestra el resultado.
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
