# Hyrulor tool-use dataset

`tool_use_dataset.jsonl` contiene ejemplos sintéticos para evaluar o preparar
una futura especialización del modelo. Cada línea es una conversación JSON con
una llamada de herramienta y su respuesta.

El dataset no sustituye las definiciones JSON que el daemon envía a
`llama-server`; esas definiciones siguen siendo obligatorias en tiempo de
ejecución.
