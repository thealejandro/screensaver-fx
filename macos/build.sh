#!/usr/bin/env bash
# Compila ScreensaverFX.saver (universal: Apple Silicon + Intel) sin proyecto de Xcode.
# Requisitos: Rust (rustup) y las Command Line Tools de Xcode (swiftc, lipo, codesign).
#   ./macos/build.sh            → target/macos/ScreensaverFX.saver
#   ./macos/build.sh --install  → además lo copia a ~/Library/Screen Savers
set -euo pipefail
cd "$(dirname "$0")/.."

export MACOSX_DEPLOYMENT_TARGET=11.0
OUT=target/macos
SAVER="$OUT/ScreensaverFX.saver"
TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)

rustup target add "${TARGETS[@]}" >/dev/null
for t in "${TARGETS[@]}"; do
  cargo build -p ssfx-core --release --target "$t"
done

mkdir -p "$OUT"
lipo -create \
  target/aarch64-apple-darwin/release/libssfx_core.a \
  target/x86_64-apple-darwin/release/libssfx_core.a \
  -output "$OUT/libssfx_core.a"

rm -rf "$SAVER"
mkdir -p "$SAVER/Contents/MacOS" "$SAVER/Contents/Resources"
for arch in arm64 x86_64; do
  swiftc -O -target "$arch-apple-macos11" -module-name ScreensaverFX \
    -import-objc-header include/ssfx.h \
    -emit-library -o "$OUT/ScreensaverFX-$arch" \
    macos/ScreensaverFXView.swift macos/ConfigSheet.swift \
    -L "$OUT" -lssfx_core \
    -framework ScreenSaver -framework AppKit -framework QuartzCore
done
lipo -create "$OUT/ScreensaverFX-arm64" "$OUT/ScreensaverFX-x86_64" -output "$SAVER/Contents/MacOS/ScreensaverFX"
cp macos/Info.plist "$SAVER/Contents/Info.plist"
# Miniaturas para la cuadrícula de Ajustes del Sistema.
cp installer/assets/thumbnail.png installer/assets/thumbnail@2x.png "$SAVER/Contents/Resources/"
codesign --force --sign - "$SAVER"
echo "Listo: $SAVER"

if [[ "${1:-}" == "--install" ]]; then
  mkdir -p "$HOME/Library/Screen Savers"
  rm -rf "$HOME/Library/Screen Savers/ScreensaverFX.saver"
  cp -R "$SAVER" "$HOME/Library/Screen Savers/"
  # Reinicia el proceso de salvapantallas para que cargue la versión nueva.
  killall legacyScreenSaver 2>/dev/null || true
  echo "Instalado en ~/Library/Screen Savers. Elígelo en Ajustes del Sistema > Salvapantallas."
fi
