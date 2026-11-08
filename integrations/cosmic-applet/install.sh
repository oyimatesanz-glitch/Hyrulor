#!/bin/sh
set -eu

PROJECT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PREFIX=${PREFIX:-"$HOME/.local"}
BIN_DIR="$PREFIX/bin"
APP_DIR="$PREFIX/share/applications"
ICON_DIR="$PREFIX/share/icons/hicolor/scalable/apps"
APP_ID="com.github.oyimatesanz-glitch.hyrulor-applet"

cd "$PROJECT_DIR"
cargo build --release --bin hyrulor-cosmic-applet

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"
install -m 0755 target/release/hyrulor-cosmic-applet "$BIN_DIR/hyrulor-cosmic-applet"
install -m 0644 "resources/icons/hicolor/scalable/apps/$APP_ID.svg" "$ICON_DIR/$APP_ID.svg"

sed "s|^Exec=.*|Exec=$BIN_DIR/hyrulor-cosmic-applet|" \
  "resources/$APP_ID.desktop" > "$APP_DIR/$APP_ID.desktop"
chmod 0644 "$APP_DIR/$APP_ID.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
fi

printf '%s\n' "Hyrulor instalado en $PREFIX" \
  "Añada '$APP_ID' desde la configuración de applets del panel COSMIC."
