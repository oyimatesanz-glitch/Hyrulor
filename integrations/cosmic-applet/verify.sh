#!/bin/sh
set -eu

cargo fmt --all -- --check
cargo test --all-targets
cargo build --release --bin hyrulor-cosmic-applet

BIN="target/release/hyrulor-cosmic-applet"
printf '\nTamaño del binario:\n'
stat -c '%n: %s bytes' "$BIN"
printf '\nDependencias dinámicas:\n'
if command -v ldd >/dev/null 2>&1; then
  ldd "$BIN" || true
else
  printf '%s\n' 'ldd no está disponible.'
fi

printf '\nCompilación y tests terminados.\n'
