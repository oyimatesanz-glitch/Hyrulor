#!/bin/sh
PATTERN=$(echo "$1" | sed -n 's/.*"pattern": *"\([^"]*\)".*/\1/p')
[ -z "$PATTERN" ] && PATTERN="fn main"

# `--` va justo antes del patrón para que un patrón que empiece por "-" no se
# interprete como opción, sin desactivar los filtros anteriores.
grep -rn \
    --include='*.rs' --include='*.md' --include='*.toml' \
    --exclude-dir=target --exclude-dir=.git --exclude-dir=.toolchain \
    --exclude-dir=node_modules --exclude-dir=logs --exclude-dir=dist \
    --exclude-dir=audit --exclude-dir=runtime --exclude-dir=models \
    --exclude-dir=archive --exclude-dir=archives --exclude-dir=.venv \
    --exclude-dir=__pycache__ \
    -- "$PATTERN" /workspace 2>/dev/null | head -n 20
