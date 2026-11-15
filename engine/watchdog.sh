#!/usr/bin/env bash
# Watchdog del Hyrulor Engine: mantiene el motor vivo y lo relanza si cae
# (crash de driver, cierre de la ventana, etc.). Se ejecuta desprendido desde
# run.sh. Para detenerlo: ./run.sh stop  (o pkill -f watchdog.sh).
set -u

ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

BIN="$ROOT/target/release/hyrulor-engine"
LOG="$ROOT/target/hyrulor-engine.log"
PIDFILE="$ROOT/target/hyrulor-engine.pid"

# Backend X11/XWayland a propósito: la integración de clipboard de Wayland
# (winit + smithay-clipboard) tiene una race en libwayland-client que da un
# SIGSEGV intermitente en COSMIC. Por X11 no existe ese hilo.
unset WAYLAND_DISPLAY
export XDG_SESSION_TYPE=x11
export RUST_LOG="${RUST_LOG:-warn}"

trap 'kill "$(cat "$PIDFILE" 2>/dev/null)" 2>/dev/null; exit 0' TERM INT

while true; do
  "$BIN" >>"$LOG" 2>&1 &
  echo $! >"$PIDFILE"
  wait "$!"
  code=$?
  echo "[watchdog $(date '+%F %T')] motor terminó (exit ${code}); reinicio en 1s" >>"$LOG"
  sleep 1
done
