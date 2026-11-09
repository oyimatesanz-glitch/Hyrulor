#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APPLET="$ROOT/integrations/cosmic-applet"
DIST="$ROOT/dist"
NAME="${1:-hyory-applet}"
OUT="$DIST/${NAME}.zip"

mkdir -p "$DIST"
rm -f "$OUT"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
mkdir -p "$STAGE/$NAME"

( cd "$APPLET" && cp -a Cargo.toml Cargo.lock LICENSE README.md THEME.md \
      install.sh verify.sh test_protocol.py .gitignore \
      fonts shaders src resources "$STAGE/$NAME/" )

( cd "$STAGE" && zip -qr "$OUT" "$NAME" )
( cd "$DIST" && sha256sum "${NAME}.zip" > "${NAME}.zip.sha256" )

echo "✅ Generado: $OUT"
echo "✅ Checksum: $DIST/${NAME}.zip.sha256"
