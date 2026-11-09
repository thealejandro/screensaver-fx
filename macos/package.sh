#!/usr/bin/env bash
# Empaqueta para distribuir: dist/ScreensaverFX-macOS.zip y dist/ScreensaverFX-<versión>.pkg
set -euo pipefail
cd "$(dirname "$0")/.."

./macos/build.sh
SAVER=target/macos/ScreensaverFX.saver
VER=$(/usr/libexec/PlistBuddy -c "Print CFBundleShortVersionString" "$SAVER/Contents/Info.plist")
WORK=target/macos/pkg
rm -rf "$WORK" && mkdir -p "$WORK/root" "$WORK/resources" dist

# Zip para instalar con doble clic o con install.sh.
ditto -c -k --sequesterRsrc --keepParent "$SAVER" dist/ScreensaverFX-macOS.zip

# Instalador .pkg con imagen de marca, bienvenida y cierre en español e inglés.
cp -R "$SAVER" "$WORK/root/"
pkgbuild --analyze --root "$WORK/root" "$WORK/component.plist"
# Si pkgbuild detecta el .saver como bundle, que no lo reubique; si no, no hay nada que cambiar.
/usr/libexec/PlistBuddy -c "Set :0:BundleIsRelocatable false" "$WORK/component.plist" 2>/dev/null || true
chmod +x installer/macos/scripts/postinstall
pkgbuild --root "$WORK/root" --component-plist "$WORK/component.plist" \
  --identifier dev.thealejandro.ScreensaverFX --version "$VER" \
  --install-location "/Library/Screen Savers" --scripts installer/macos/scripts \
  "$WORK/component.pkg"
cp -R installer/macos/resources/. "$WORK/resources/"
cp installer/assets/pkg-background.png installer/assets/pkg-mark.png "$WORK/resources/"
sed "s/__VERSION__/$VER/" installer/macos/distribution.xml > "$WORK/distribution.xml"
productbuild --distribution "$WORK/distribution.xml" --resources "$WORK/resources" \
  --package-path "$WORK" "dist/ScreensaverFX-$VER.pkg"
cp "dist/ScreensaverFX-$VER.pkg" dist/ScreensaverFX.pkg
echo "Listo: dist/ScreensaverFX-macOS.zip y dist/ScreensaverFX-$VER.pkg"
