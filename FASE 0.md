Perfecto: Fase 0 completa, sin exclusivas de Electron. Aquí va el plan detallado.
Plan: Fase 0 — Sistema y ventana
1. Icono de ventana
- assets/icon.png (256×256, orbe dorado sobre fondo oscuro; lo genero con un script Rust de 50 líneas usando image, o lo creas tú).
- main.rs: Window::set_window_icon tras la creación, cargado con include_bytes! + decodificación png.
2. F11 fullscreen + atajos base
- WindowEvent::KeyboardInput → Fullscreen::Borderless(None) toggle.
- Base para el resto de atajos (F12, Esc).
3. HiDPI / Retina
- Leer window.scale_factor() en cada evento Resized.
- Recrear SurfaceConfiguration con width = phys_w / scale, height = phys_h / scale y pasar la escala a egui (Context::set_zoom_factor o egui_winit::State que ya lo gestiona).
- Validar en monitor externo con escala ≠ 1.
4. Límite de FPS
- Pacing en winit ApplicationHandler::new_event/bucle: calcular target_frame = 1/60 (configurable por config) y thread::sleep la diferencia tras present().
- Toggle F3 para on/off (útil para medir rendimiento).
5. tracing + panic hook
- Reemplazar env_logger por tracing + tracing-subscriber (JSON opcional, niveles por módulo: hyrulor_engine::daemon=debug).
- Panic hook: capturar Box<dyn Any> → escribir con backtrace en ~/.local/state/hyrulor/engine.log (dirs dirs-rs) y mostrar error en pantalla (pantalla roja con el mensaje, sin crash a negro).
- Rota el log a 5 MB, mantiene 3 generaciones.
6. config.toml
- ~/.config/hyrulor/engine.toml con defaults documentados:
[window]  width, height, min_w, min_h, title, fps_cap, vsync
[theme]   mode = "auto"  # auto | light | dark, density, accent
[audio]   ui_sounds = true
[log]     level = "info"
- Crates: toml, dirs, notify (hot-reload: tema, fps, sonidos aplican sin reinicio; tamaño de ventana sí requiere reinicio, se avisa en toasts).
7. Bus de eventos interno (emit/listen)
- EventBus sobre std::sync::mpsc broadcast o crossbeam: emit("daemon:status", payload), subscribe("daemon:*").
- Desacopla el cliente del daemon de la UI: el daemon publica, egui suscribe. Es la semilla para F12, toasts y el inspector.
8. Detección GPU + info
- adapter_info() tras wgpu::request_adapter: nombre, tipo (Discrete/Integrated/Cpu), VRAM.
- HUD opcional (tecla F3) con: GPU, fps real, ms de frame (CPU/GPU), draw calls.
- Si el adapter es Backend::Gl → aviso "modo software (llvmpipe)".
- Fallback: si request_adapter devuelve None, log claro + pantalla de error.
9. Pausa al minimizar
- WindowEvent::Focused(false) + WindowEvent::Occluded → congelar el reloj del orbe (no avanzar elapsed) y saltar renders hasta volver; no bajar a N FPS (más simple y suficiente).
10. Drag & drop
- WindowEvent::DragDropPath de winit (Linux: requiere que el binary declare MIME types — usar data_uri/mime::TEXT_PLAIN y validar; si el entorno Wayland no lo soporta, log warn y sin romper).
- Usos concretos: soltar un .json de historial → cargar en chat; soltar imagen → (Fase 7, placeholder por ahora).
11. Tema del sistema
- Leer XDG_CURRENT_DESKTOP/config de GTK (gsettings org.gnome.desktop.interface color-scheme) en Linux, NSRequiresAquaSystemAppearance en macOS, DwmGetWindowAttribute en Windows.
- Modo auto de la config lo consume; light/dark forzado.
- Genera dos egui::Visuals (oscuro ya existe implícito, claro nuevo) con la paleta oro/negro.
12. Captura F12
- Readback: TextureView de la swapchain → StagingBuffer → write_buffer → mapear → image::save_buffer PNG a ~/Pictures/hyrulor-captura-<ts>.png.
- Wgpu ya soporta readback de texturas renderizadas; hay que copiar a textura TextureUsages::COPY_SRC (la post-procesada) antes del present.
13. CLI args
- clap: --socket <path>, --config <path>, --fps <n>, --headless-test (arranca, comprueba daemon, imprime estado y sale — útil para CI), --version, --verbose.
14. Splash screen
- Durante los primeros frames (hasta la 1ª respuesta de salud del daemon, máx. 3 s): egui dibuja orbe animado + "conectando…" con el estilo oficial, sin mostrar el chat aún.
- Si el daemon no responde en 3 s: pasar a la ventana normal con badge "sin conexión" (comportamiento actual, sin delay).
15. Multi-ventana (mínimo)
- Una 2ª ventana winit (tecla F4 o --window monitor): panel del daemon (status, latencia, últimas 50 peticiones, GPU info).
- Ambas comparten el mismo Arc<Mutex<EngineState>>; la 2ª solo renderiza egui (sin wgpu del orbe, más barata).
16. Profiling por frame
- tracing::span! por frame (begin/end) + contador de Instant en: input, lógica, wgpu submit, present.
- HUD F3 muestra los 4 tiempos + p95 de los últimos 60 frames.
- Sin tracing-chrome export por ahora (se añade si se necesita).
17. Cursor
- set_cursor_visible(false) sobre zonas de UI "estéticas" (p. ej. el orbe cuando el chat no tiene foco) + cambiar icono de cursor según widget (egui ya emite CursorIcon).
18. Touch/multitouch
- winit ya emite DeviceEvent::Touch/WindowEvent::Touch (en Wayland con protocolo touch). Validar que egui los consuma (soporta PointerButton genérico); si falta un mapping, lo añado en el handler.
19. Empaquetado
- Containerfile.engine (nuevo, junto al Containerfile.tools existente de hyrulor/): imagen mínima con runtime Vulkan.
- hyrulor-engine.flatpakref + spec AppImage (via appimagetool) en hyrulor-engine/packaging/.
- Smoke-test: --headless-test dentro de la imagen.
20. VSync / resolución dinámica (de la lista original)
- VSync on por defecto (PresentMode::Fifo), --fps lo desactiva (VsyncOff + pacing manual).
- "Resolución dinámica" la dejo fuera: no aporta nada con wgpu + orbe, más complexity. (Se puede reabrir después.)
Orden de ejecución sugerido (dependencias)
1. Config + tracing + panic hook (todo lo demás usa log y config)
2. HiDPI + FPS cap + VSync (tocan el mismo bloque de SurfaceConfiguration)
3. Icono + F11 + cursor + pausa (eventos de ventana, un mismo patch)
4. Detección GPU + HUD F3 + profiling
5. Bus de eventos + splash + F12 + drag&drop
6. Tema del sistema + claro/oscuro
7. CLI + headless-test + multi-ventana
8. Packaging (AppImage/Flatpak/Containerfile)
Crates nuevas
clap, toml, dirs, notify, tracing, tracing-subscriber, image, png (o solo image), crossbeam (bus), rodio (lo dejo para Fase 6).
Riesgos
- Drag&drop en Wayland: soportado vía winit solo en X11 por defecto; en Wayland depende del compositor. Mitigación: degradar a log warn, no romper.
- Readback F12: en backends con srgb8alpha8 requiere TEXTURE_BINDING → COPY_SRC copy; si el pipeline post no expone la textura, se hace un segundo render_pass de copia. 1-2 h de trabajo.
- Tema auto en Wayland: no hay API estandarizada; el fallback es "si no se detecta, dark".
- Multi-ventana: winit 0.30 soporta varios Window en un event loop, pero egui_winit por ventana → dos State egui. Manejable.
¿Sigo con este plan tal cual (lo paso a un archivo en /home/oyi/.opencode/plan si quieres que lo persista), o ajustamos algo primero? Por ejemplo: ¿el monitor multi-ventana (ítem 15) lo quieres en Fase 0 o lo dejamos para Fase 5?
Plan · Qwen3.8 27B · 8m 42s · 18.3 tok/s
Flutter	Dart	Motor gráfico propio (Impeller/Skia)	UI idéntica y nativa en Android, iOS, Windows y Mac con un solo código.
Dioxus	Rust	WebView o Renderizado Nativo	Sintaxis tipo React nativa en Rust, sin necesidad de separar frontend y backend.
Electron	JavaScript / TypeScript	Chromium integrado	Ecosistema gigantesco de plugins y compatibilidad total garantizada en Linux/Mac/Windows.
Wails	Go (Golang)	WebView del sistema	Reemplaza Rust por Go en el backend conservando la ligereza de Tauri.
GPUI	Rust	Aceleración por GPU nativa	Rendimiento masivo de nivel videojuego (es el motor del editor Zed). 
+Thought · 1m 21s
Buena tabla. Mi lectura honesta: tus opciones se dividen en "cambio de stack" (los 5) vs "seguir con lo tuyo", y en tu caso concreto hay un matiz clave:
Lo que ningún framework de esa lista te da
El orbe. E3–E7 (esfera 3D, shader Fresnel/escaneo/pulso, bloom, estado codificado en el orbe) son tu diferenciador y son ~1.800 líneas de wgpu crudo + WGSL. En cualquiera de los 5 tendrías que:
- Flutter: embeber el shader en un CustomPainter/FBO hack, o usar un package de escena 3D (three_dart) que no tiene nada que ver con tu pipeline.
- Dioxus: su renderer nativo no soporta escenas 3D arbitrarias; iría en webview o en un webview_raw incrustado.
- Electron/Wails: Three.js/Babylon en un canvas web, o un externalBin que arranca tu wgpu en otra ventana (así no es una app).
- GPUI: es un framework de UI 2D acelerado por GPU, no un motor 3D. El orbe seguiría siendo "tu wgpu aparte".
Análisis uno a uno
Framework	Veredicto para Hyrulor
Flutter/Dart	❌ Reescritura completa en Dart. El daemon es Rust y habla JSON-Lines; el cliente en Dart es posible pero pierdes la tipificación compartida y todo lo compilado. Su UI es homogénea entre plataformas, sí, pero para un app de orbe+chat no necesitas esa homogeneidad: egui ya se ve nativo en los 3 SO.
Dioxus	⚠️ El único candidato serio por ser Rust + sintaxis React. Pero: su target desktop sigue siendo webview (Tauri por debajo), y su renderer nativo (dioxus-wgpu) es inmaduro y no soporta tus shaders 3D. Te darías exactamente el problema que evitaste al abandonar el webview. Solo vale si quieres múltiples apps de UI en Rust reutilizando componentes — hoy no es tu caso.
Electron	❌ Chromium + Node. 200-400 MB de RAM, reescritura en TS, y el "ecosistema gigante" de que se presume aquí es plugins para patrones web (auth OAuth, Stripe…) que no usas.
Wails	❌ Te obliga a Go en el backend. Tu ecosistema entero (daemon, inferencia, motor) es Rust. Incompatible.
GPUI	⚠️ El más "espíritu affine" de la lista (Rust, GPU, nativo). Pero: (1) es un framework de UI 2D — el orbe seguiría siendo tu pipeline aparte, (2) su API cambia rápido (nace con Zed), (3) integrarlo con tu wgpu propio implica pelear por el device/surface, (4) migrar = reescribir main.rs completo (~1.300 líneas). Su verdadero valor (Zed) es un editor con miles de widgets; tú tienes chat + formularios + HUD, que egui cubre ya.
Mi recomendación
Seguir con winit + wgpu + egui. Es, funcionalmente, la categoría "GPUI de la lista" que ya tienes construida y dominando: GPU nativa, Rust, sin webview. La tabla que pegaste compara frameworks que empiezan de cero; tú ya estás en el punto donde cada uno de ellos empieza (motor propio nativo) pero con el 3D que ninguno ofrece.
Cambio de stack = pagar reescritura y perder el orbe. Quedarse = el plan de Fase 0 que ya tienes sobre la mesa (20 ítems, ~2-3 semanas), que es lo único que realmente te falta.
La única cosa de esa tabla que sí copiaría: la disciplina de GPUI — medir con profiling por frame y no añadir widgets sin midir. Eso va dentro del ítem 16 del plan (HUD F3 + tracing).
¿Mantengo el plan de Fase 0 como está y lo paso a archivo en /home/oyi/.opencode/plan?