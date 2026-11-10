#!/usr/bin/env bash
# Empaqueta para distribuir: dist/ScreensaverFX-macOS.zip, dist/ScreensaverFX-<versión>.pkg y .dmg
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

# .dmg para instalar arrastrando: el .saver y un acceso a /Library/Screen Savers (para todos;
# Finder pide la contraseña). Fondo y posiciones salen de core/examples/brand.rs (DMG_ICONS).
DMG=target/macos/dmg
VOL="Screensaver FX"
rm -rf "$DMG" && mkdir -p "$DMG/src/.background"
cp -R "$SAVER" "$DMG/src/"
ln -s "/Library/Screen Savers" "$DMG/src/Screen Savers"
tiffutil -cathidpicheck installer/assets/dmg-background.png installer/assets/dmg-background@2x.png \
  -out "$DMG/src/.background/background.tiff"
hdiutil create -quiet -volname "$VOL" -srcfolder "$DMG/src" -fs HFS+ -format UDRW -ov "$DMG/rw.dmg"
# Si ya hubiera uno montado con el mismo nombre, Finder no sabría cuál decorar.
[ -d "/Volumes/$VOL" ] && hdiutil detach -quiet "/Volumes/$VOL" || true
# Salida de attach: «/dev/diskNsM<TAB>Apple_HFS<TAB>/Volumes/…». Se trabaja con el punto de montaje
# real: si el detach anterior falló, macOS lo habrá montado como «Screensaver FX 1».
ATTACH=$(hdiutil attach -readwrite -noverify -noautoopen "$DMG/rw.dmg" | grep Apple_HFS)
DEV=$(printf '%s' "$ATTACH" | awk '{print $1}')
MNT=$(printf '%s' "$ATTACH" | awk -F'\t' '{print $NF}')
# Diseño de la ventana con Finder. Sin sesión gráfica (o si Finder no responde en 90 s) el .dmg
# sigue siendo válido, solo que sin fondo ni posiciones. Las posiciones son el centro de cada
# icono; la ventana mide 640x428 = fondo de 640x400 + 28 de barra de título.
osascript >/dev/null <<EOF &
tell application "Finder"
  tell disk "$(basename "$MNT")"
    open
    set current view of container window to icon view
    set toolbar visible of container window to false
    set statusbar visible of container window to false
    set the bounds of container window to {200, 120, 840, 548}
    set o to the icon view options of container window
    set arrangement of o to not arranged
    set icon size of o to 112
    set text size of o to 13
    set background picture of o to file ".background:background.tiff"
    set position of item "ScreensaverFX.saver" of container window to {170, 235}
    set position of item "Screen Savers" of container window to {470, 235}
    close
    open
    update without registering applications
    delay 2
    close
  end tell
end tell
EOF
OSA=$!
( sleep 90; kill $OSA; sleep 5; kill -9 $OSA ) >/dev/null 2>&1 &
WATCH=$!
wait $OSA && echo "Ventana del .dmg diseñada" || echo "aviso: Finder no diseñó la ventana del .dmg"
kill $WATCH 2>/dev/null || true
rm -rf "$MNT/.fseventsd"
sync
hdiutil detach -quiet "$DEV" || hdiutil detach -quiet -force "$DEV"
rm -f "dist/ScreensaverFX-$VER.dmg"
hdiutil convert -quiet "$DMG/rw.dmg" -format UDZO -imagekey zlib-level=9 -o "dist/ScreensaverFX-$VER.dmg"
cp "dist/ScreensaverFX-$VER.dmg" dist/ScreensaverFX.dmg
echo "Listo: dist/ScreensaverFX-macOS.zip, dist/ScreensaverFX-$VER.pkg y dist/ScreensaverFX-$VER.dmg"
