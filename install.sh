#!/usr/bin/env bash
# Screensaver FX: instalación desde la terminal (macOS 11 o posterior, sin contraseña).
#   curl -fsSL https://raw.githubusercontent.com/thealejandro/screensaver-fx/main/install.sh | bash
#   curl -fsSL …/install.sh | bash -s -- --uninstall
set -euo pipefail
REPO=thealejandro/screensaver-fx
DEST="$HOME/Library/Screen Savers"
SAVER="$DEST/ScreensaverFX.saver"
say() { printf '  \033[36m%s\033[0m\n' "$1"; }

printf '\n  \033[35m❯ Screensaver FX\033[0m\n\n'
if [[ "$(uname)" != Darwin ]]; then
  echo "  Este script es para macOS. En Windows usa install.ps1."
  exit 1
fi

if [[ "${1:-}" == "--uninstall" ]]; then
  rm -rf "$SAVER"
  killall legacyScreenSaver 2>/dev/null || true
  say "Desinstalado."
  exit 0
fi

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
say "Descargando la última versión…"
curl -fsSL "https://github.com/$REPO/releases/latest/download/ScreensaverFX-macOS.zip" -o "$TMP/saver.zip"
ditto -x -k "$TMP/saver.zip" "$TMP"
say "Instalando en ~/Library/Screen Savers…"
mkdir -p "$DEST"
rm -rf "$SAVER"
mv "$TMP/ScreensaverFX.saver" "$DEST/"
# Lo descargado con curl no lleva cuarentena; por si acaso se quita.
xattr -dr com.apple.quarantine "$SAVER" 2>/dev/null || true
killall legacyScreenSaver 2>/dev/null || true
say "Instalado. Elígelo en Ajustes del Sistema › Salvapantallas."
open "x-apple.systempreferences:com.apple.ScreenSaver-Settings.extension" 2>/dev/null || true
echo
