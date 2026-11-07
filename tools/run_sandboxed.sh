#!/usr/bin/env bash
# Uso: ./run_sandboxed.sh /ruta/al/script.sh
SCRIPT="$1"

exec bwrap \
  --ro-bind /usr /usr \
  --ro-bind /lib /lib \
  --ro-bind /lib64 /lib64 \
  --ro-bind /bin /bin \
  --ro-bind /etc /etc \
  --ro-bind "$(dirname "$SCRIPT")" /scripts \
  --tmpfs /tmp \
  --proc /proc \
  --dev /dev \
  --unshare-all \
  --share-net \
  --die-with-parent \
  --new-session \
  --chdir /tmp \
  /bin/bash /scripts/"$(basename "$SCRIPT")"
