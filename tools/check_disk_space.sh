#!/bin/sh
# /usr está bindeado desde el host, así que df muestra el disco real.
echo "Host root filesystem:"
df -h /usr 2>&1 | head -n 5
