#!/bin/sh
FILE=$(echo "$1" | sed -n 's/.*"path": *"\([^"]*\)".*/\1/p')
[ -z "$FILE" ] && FILE="src/main.rs"
if [ -f "/workspace/$FILE" ]; then
  head -n 120 "/workspace/$FILE"
else
  echo "Error: no existe /workspace/$FILE"
fi
