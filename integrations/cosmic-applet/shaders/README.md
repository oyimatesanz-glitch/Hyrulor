# Shaders (siguiente iteración)

La applet de esta entrega no compila ni ejecuta shaders. El renderizado se mantiene en el camino nativo estándar de libcosmic para no añadir una ruta gráfica paralela sin validar.

Para activar aurora, polvo y parallax por GPU hace falta habilitar `wgpu` en libcosmic y desarrollar un widget/renderer iced compatible con la versión exacta del toolkit. Añadir únicamente un archivo WGSL no produce un efecto visible ni garantiza consumo cero de CPU/RAM.
