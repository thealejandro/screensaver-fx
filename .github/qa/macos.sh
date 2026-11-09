#!/usr/bin/env bash
# QA de los paquetes de macOS en una máquina limpia (CI): .pkg, .zip, arquitecturas, firma y dibujo.
#   .github/qa/macos.sh dist
set -uo pipefail
DIST=${1:-dist}
fails=0
check() { if eval "$2"; then echo "  OK    $1"; else echo "  FALLA $1"; fails=$((fails + 1)); fi; }

echo "Máquina: $(uname -m) · macOS $(sw_vers -productVersion)"

echo "1) .pkg"
check "el paquete está firmado o al menos es válido" "pkgutil --check-signature '$DIST/ScreensaverFX.pkg' >/dev/null 2>&1 || pkgutil --payload-files '$DIST/ScreensaverFX.pkg' >/dev/null"
check "instala sin errores" "sudo installer -pkg '$DIST/ScreensaverFX.pkg' -target / >/tmp/installer.log 2>&1 || { cat /tmp/installer.log; false; }"
SAVER="/Library/Screen Savers/ScreensaverFX.saver"
check "queda en /Library/Screen Savers" "[ -d '$SAVER' ]"
BIN="$SAVER/Contents/MacOS/ScreensaverFX"
check "binario universal (arm64 + x86_64)" "lipo -archs '$BIN' | grep -q arm64 && lipo -archs '$BIN' | grep -q x86_64"
check "firma ad hoc válida" "codesign --verify --deep --strict '$SAVER'"
check "NSPrincipalClass correcto" "[ \"\$(/usr/libexec/PlistBuddy -c 'Print NSPrincipalClass' '$SAVER/Contents/Info.plist')\" = ScreensaverFXView ]"
check "miniaturas incluidas" "[ -f '$SAVER/Contents/Resources/thumbnail.png' ] && [ -f '$SAVER/Contents/Resources/thumbnail@2x.png' ]"
check "macOS mínimo 11" "otool -l '$BIN' | grep -A3 LC_BUILD_VERSION | grep -q 'minos 11'"

echo "2) Carga y dibujo como lo hace macOS ($(uname -m))"
swift .github/qa/smoke.swift "$SAVER" || fails=$((fails + 1))

echo "3) .zip (instalación de usuario)"
rm -rf /tmp/zipqa && mkdir -p /tmp/zipqa
check "se descomprime" "ditto -x -k '$DIST/ScreensaverFX-macOS.zip' /tmp/zipqa"
check "contiene el .saver con la misma firma" "codesign --verify --deep --strict /tmp/zipqa/ScreensaverFX.saver"

echo "Resultado: $fails fallos"
exit $fails
