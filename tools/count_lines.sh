#!/bin/sh
FILE=$(echo "$1" | sed -n 's/.*"path": *"\([^"]*\)".*/\1/p')
wc -l "$FILE" 2>/dev/null || echo "Error"
