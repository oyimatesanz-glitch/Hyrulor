#!/usr/bin/env bash
# Lanzador del Hyrulor Engine.
#
# Arranca el motor por debajo de un watchdog (watchdog.sh) que lo relanza si
# cae, usando el backend X11/XWayland para evitar la race de clipboard de
# Wayland (SIGSEGV intermitente en libwayland-client).
#
# Uso:
#   ./run.sh        # arranca el motor (y lo mantiene vivo)
#   ./run.sh stop   # detiene watchdog y motor
set -u

ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

BIN="$ROOT/target/release/hyrulor-engine"
WDPID="$ROOT/target/hyrulor-engine-watchdog.pid"
mkdir -p "$ROOT/target"

stop() {
  pkill -f "watchdog.sh" 2>/dev/null || true
  pkill -x hyrulor-engine 2>/dev/null || true
  rm -f "$WDPID" "$ROOT/target/hyrulor-engine.pid"
  echo "Motor y watchdog detenidos."
}

case "${1:-}" in
  stop) stop; exit 0 ;;
  status)
    pkill -0 -f "watchdog.sh" 2>/dev/null && echo "watchdog: VIVO" || echo "watchdog: caído"
    pkill -0 -x hyrulor-engine 2>/dev/null && echo "motor: VIVO (pid $(cat "$ROOT/target/hyrulor-engine.pid" 2>/dev/null))" || echo "motor: caído"
    exit 0
    ;;
esac

[ -x "$BIN" ] || { echo "Binario no encontrado: $BIN — ejecuta 'cargo build --release'"; exit 1; }

# Reiniciar limpio.
stop

# Arrancar el watchdog desprendido (sobrevive a esta shell y al cerrarla).
setsid nohup bash "$ROOT/watchdog.sh" >>"$ROOT/target/watchdog.log" 2>&1 &
WPID=$!
echo "$WPID" >"$WDPID"

sleep 2
MPID=$(cat "$ROOT/target/hyrulor-engine.pid" 2>/dev/null || echo "?")
if pkill -0 -x hyrulor-engine 2>/dev/null; then
  echo "Motor en marcha (watchdog pid=$WPID, motor pid=$MPID)."
else
  echo "⚠️ El watchdog arrancó (pid=$WPID) pero el motor aún no está vivo."
fi
echo "  Log:    $ROOT/target/hyrulor-engine.log"
echo "  Parar:  $0 stop"
