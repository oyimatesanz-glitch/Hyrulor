#!/bin/sh
DIR=$(echo "$1" | sed -n 's/.*"path": *"\([^"]*\)".*/\1/p'); [ -z "$DIR" ] && DIR="."
ls -lah "$DIR" 2>/dev/null || echo "Error: ruta no encontrada"
