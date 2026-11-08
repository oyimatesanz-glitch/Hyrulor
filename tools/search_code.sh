#!/bin/sh
PATTERN=$(echo "$1" | sed -n 's/.*"pattern": *"\([^"]*\)".*/\1/p')
[ -z "$PATTERN" ] && PATTERN="fn main"
grep -rn "$PATTERN" /workspace --include='*.rs' --include='*.md' --include='*.toml' 2>/dev/null | head -n 20
