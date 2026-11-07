# Hyrulor — Digital Aristocracy

## Design tokens

| Token | Color | Uso |
|---|---|---|
| Forest | `#0E1711` | Fondo principal |
| Forest card | `#17251C` | Superficies elevadas |
| Assistant | `#1E2A22` | Burbujas del asistente |
| User | `#142018` | Burbujas del usuario |
| Brass | `#B08D57` | Marcos, acciones y acentos |
| Champagne | `#D9BE86` | Titulares |
| Cream | `#E8DFC9` | Texto principal |
| Sage | `#7A9E7E` | Estado operativo |
| Oxblood | `#5A1B25` | Estado de error |
| Pewter | `#999C90` | Texto secundario |

## Tipografía

Objetivo visual: Instrument Serif/Cormorant Garamond para títulos, Inter/Geist para texto y JetBrains Mono para datos técnicos. La applet no descarga fuentes. Esta entrega todavía usa la selección tipográfica del sistema; `fonts/README.md` explica cómo completar el empaquetado local con archivos de licencia adecuada.

## Geometría

- Popup objetivo: 420 × 620 px.
- Esquinas: cuadradas en esta implementación; no se introducen radios superiores a 4 px.
- Marcos: doble capa de latón y crema mediante contenedores anidados.
- Burbujas: superficies verde bosque, con marco fino de latón.
- Densidad: márgenes contenidos y jerarquía tipográfica discreta.

## Movimiento y efectos

Implementado: pulso discreto del monograma mientras se espera una respuesta (suscripción temporal de 900 ms); el ciclo se detiene en reposo.

Pendiente de integración gráfica: giro de 360°, anillo de herramienta, shake de error, aparición palabra a palabra, pincelada de tinta animada, shimmer de hover, parallax, polvo ambiental y aurora. No se simulan como animaciones CSS: una integración real necesita widgets/renderer nativos y, para los efectos de GPU, la ruta `wgpu`/WGSL de iced. La applet estándar de libcosmic no convierte automáticamente un shader suelto en un efecto de interfaz.
