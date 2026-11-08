#!/bin/sh
FILE=$(echo "$1" | sed -n 's/.*"path": *"\([^"]*\)".*/\1/p')
if [ -z "$FILE" ]; then echo "Error: falta el parámetro path"; exit 1; fi
head -n 100 "$FILE" 2>/dev/null || echo "Error al leer archivo"
