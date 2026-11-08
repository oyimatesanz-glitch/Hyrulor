#!/bin/sh
set -eu

cargo fmt --all -- --check
cargo test --all-targets
cargo build --release --bin hyrulor-cosmic-applet --bin hyrulor-mock-daemon

BIN="target/release/hyrulor-cosmic-applet"
MOCK="target/release/hyrulor-mock-daemon"
SOCKET="${TMPDIR:-/tmp}/hyrulor-verify-$$.sock"

printf '\nPrueba de protocolo contra el daemon simulado:\n'
HYORY_SOCKET="$SOCKET" "$MOCK" &
MOCK_PID=$!
cleanup() {
  kill "$MOCK_PID" 2>/dev/null || true
  rm -f "$SOCKET"
}
trap cleanup EXIT INT TERM

for _ in $(seq 1 50); do
  [ -S "$SOCKET" ] && break
  sleep 0.1
done

if [ ! -S "$SOCKET" ]; then
  printf '%s\n' 'El daemon simulado no creó el socket.' >&2
  exit 1
fi

HYORY_SOCKET="$SOCKET" python3 test_protocol.py
cleanup
trap - EXIT INT TERM

printf '\nTamaño del binario:\n'
stat -c '%n: %s bytes' "$BIN"
printf '\nDependencias dinámicas:\n'
if command -v ldd >/dev/null 2>&1; then
  ldd "$BIN" || true
else
  printf '%s\n' 'ldd no está disponible.'
fi

printf '\nCompilación, tests y protocolo terminados.\n'
