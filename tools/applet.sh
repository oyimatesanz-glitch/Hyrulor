#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APPLET="$ROOT/integrations/cosmic-applet"

case "${1:-help}" in
  build)
    cargo build --release --manifest-path "$APPLET/Cargo.toml" \
      --bin hyrulor-cosmic-applet --bin hyrulor-mock-daemon
    ;;
  test)
    cargo test --manifest-path "$APPLET/Cargo.toml" --all-targets
    ;;
  fmt)
    cargo fmt --manifest-path "$APPLET/Cargo.toml" --all -- --check
    ;;
  verify)
    "$0" fmt && "$0" test
    ;;
  run)
    cargo run --manifest-path "$APPLET/Cargo.toml" --bin hyrulor-cosmic-applet
    ;;
  mock)
    cargo run --manifest-path "$APPLET/Cargo.toml" --bin hyrulor-mock-daemon
    ;;
  install)
    ( cd "$APPLET" && ./install.sh )
    ;;
  *)
    echo "uso: $0 {build|test|fmt|verify|run|mock|install}"; exit 1
    ;;
esac
