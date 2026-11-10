#!/bin/sh
FILE=$(echo "$1" | sed -n 's/.*"path": *"\([^"]*\)".*/\1/p')
[ -z "$FILE" ] && FILE="src/main.rs"

# No permitir rutas absolutas ni "..": el tool solo puede leer bajo /workspace.
case "$FILE" in
    /*|..|../*|*/..|*/../*)
        echo "Error: ruta fuera de /workspace: $FILE"
        exit 1
        ;;
esac

if [ -f "/workspace/$FILE" ]; then
    head -n 120 "/workspace/$FILE"
else
    echo "Error: no existe /workspace/$FILE"
fi
