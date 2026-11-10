# Hyrulor — Obsidian & Gold

Tokens derived from `/home/oyi/Proyectos/designs/index.html` (the Hyrulor
landing page and chat mock-up).

## Design tokens

| Token | Color | Uso |
|---|---|---|
| Background | `#050505` | Fondo del popup |
| Glass | `rgba(10, 10, 10, 0.82)` | Superficie principal translúcida |
| Card | `rgba(20, 20, 20, 0.72)` | Tarjetas y paneles elevados |
| Assistant bubble | `rgba(20, 20, 20, 0.68)` | Burbujas del asistente |
| User bubble | `rgba(212, 175, 55, 0.08)` | Burbujas del usuario |
| Tool bubble | `rgba(0, 212, 255, 0.07)` | Registros de herramientas |
| Gold | `#D4AF37` | Marcos, títulos y acciones |
| Gold light | `#FFD700` | Hover y texto de acento |
| Gold dark | `#8B6914` | Estados pulsados |
| Cyan | `#00D4FF` | Datos técnicos y herramientas |
| Text primary | `#F8F9FA` | Texto principal |
| Text secondary | `#9CA3AF` | Texto de apoyo |
| Text muted | `#6B7280` | Metadatos |
| Success | `#00FF88` | Estado local listo |
| Danger | `#FF4444` | Errores |
| Border subtle | `rgba(255, 255, 255, 0.08)` | Separadores y marcos |
| Border gold | `rgba(212, 175, 55, 0.30)` | Marcos de acento |
| Border cyan | `rgba(0, 212, 255, 0.28)` | Marcos de herramienta |

## Tipografía

Objetivo visual: Orbitron para el logotipo, Space Grotesk/Inter para texto y
JetBrains Mono para datos técnicos. La applet no descarga fuentes; usa la
selección tipográfica del sistema. `fonts/README.md` explica cómo completar el
empaquetado local con archivos de licencia adecuada.

## Geometría

- Popup: 420 × 620 px.
- Radio del panel principal: 28 px; tarjetas y burbujas: 20 px; cajas internas: 12–14 px.
- Botones y campos: píldoras de 100 px de radio.
- Marco: una capa de 1 px (`border_subtle` / `border_gold`).
- Densidad: márgenes contenidos, jerarquía tipográfica discreta.

## Movimiento y efectos

Implementado: el punto de estado del encabezado alterna entre oro y oro claro
mientras el asistente razona (suscripción temporal de 900 ms); el ciclo se
detiene en reposo.

Pendiente de integración gráfica: giro de 360°, anillo de herramienta, shake de
error, aparición palabra a palabra, pincelada de tinta animada, shimmer de
hover, parallax, polvo ambiental y aurora. No se simulan como animaciones CSS:
una integración real necesita widgets/renderer nativos y, para los efectos de
GPU, la ruta `wgpu`/WGSL de iced. La applet estándar de libcosmic no convierte
automáticamente un shader suelto en un efecto de interfaz.
