#!/bin/sh
PATTERN=$(echo "$1" | sed -n 's/.*"name": *"\([^"]*\)".*/\1/p'); [ -z "$PATTERN" ] && PATTERN="*.rs"
find . -name "$PATTERN" -type f 2>/dev/null | head -n 20
