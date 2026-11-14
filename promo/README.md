# Vídeos promocionales de Screensaver FX

Las piezas de vídeo de Screensaver FX, hechas con [Remotion](https://www.remotion.dev). Cada pieza existe en
español y en inglés:

| Pieza | Tamaño | Duración |
| --- | --- | --- |
| Reel | 1080 × 1920 | 29,6 s |
| Horizontal | 1920 × 1080 | 52,8 s |
| Estado | 1080 × 1920 | 16,8 s |

Además salen una portada (1080 × 1920), un póster (1920 × 1080), una versión para WhatsApp del reel y del
estado (menos de 25 MB), una versión web del reel a 720p (8 MB como mucho, con su póster) y un clip corto en
inglés para el README (MP4 y GIF, sin audio).

Las animaciones que se ven son las del motor real (`core/`), no una imitación. Las ventanas de configuración
de Windows y de macOS y la terminal son recreaciones con las cadenas reales del código, van marcadas como
«Recreación» y no llevan logotipos de ningún sistema.

## Uso

Requisitos: Node 20 o posterior, Rust (cargo), Python 3 con NumPy y ffmpeg.

```bash
cd promo
npm install
npm run clips       # clips del motor real en public/clips/ (compila core/examples/render.rs)
npm run typecheck
npm run studio      # vista previa
npm run build       # todas las piezas en out/ y el informe de calidad en out/qa/
```

`npm run clips -- --solo v/` rehace solo los clips verticales y `--forzar` los vuelve a generar aunque ya
existan. Los clips son deterministas (misma semilla y misma hora del reloj) y no se versionan.

### Salidas

- `out/screensaver-fx-<reel|wide|estado>-<es|en>.mp4`: 60 fps.
- `out/screensaver-fx-<reel|estado>-<es|en>-whatsapp.mp4`: 30 fps, menos de 25 MB.
- `out/web/screensaver-fx-720p[-en].mp4` y `out/web/screensaver-fx-poster[-en].jpg`: el reel para la web.
- `out/screensaver-fx-cover-<es|en>.jpg` y `out/screensaver-fx-poster-<es|en>.jpg`.
- `out/readme/screensaver-fx-en.mp4` y `.gif`: los 12 primeros segundos del horizontal en inglés.
- `out/qa/`: sonoridad (EBU R128) y un mosaico de fotogramas de cada MP4.

## Estructura

- `src/timeline.json`: escenas de cada formato en pulsos de la música (150 BPM, 24 fotogramas por pulso) y sus
  marcas. La usan el vídeo y la mezcla, así que los cortes caen siempre en el pulso.
- `src/escenas/`: las escenas. `src/textos.ts` tiene todos los textos en los dos idiomas.
- `scripts/clips.py`: pide los fotogramas al motor y los escala con píxel nítido.
- `scripts/audio/`: la música y la mezcla.
- `public/arte.txt`: el arte ASCII de ejemplo (una taza original), que hace de `screensaver.txt`.

## Música

La música es propia y se genera con el motor de `scripts/audio/motor/`: muestras grabadas de instrumentos
reales, sin IA. Es un motivo original en re mayor a 150 BPM para balafón, strumstick, marimba, cajón y palmas,
con las palmas en el 2 y el 4. Cada pieza tiene un respiro que sube hasta un corte seco. El reel y el
horizontal van a −15 LUFS y el estado a −18 LUFS, con el pico real a −1 dBTP como mucho.

La mezcla ya está en `public/audio/`. Para rehacerla hacen falta las muestras, que no se versionan.

```bash
cd promo
# Muestras VCSL (CC0), solo las carpetas que se usan
git clone --filter=blob:none --sparse https://github.com/sgossner/VCSL.git muestras/vcsl
git -C muestras/vcsl sparse-checkout set \
  "Idiophones/Struck Idiophones/Balafon/Soft Mallet" \
  "Chordophones/Composite Chordophones/Strumstick/Finger" \
  "Idiophones/Struck Idiophones/Marimba" \
  "Idiophones/Struck Idiophones/Cajon" \
  "Idiophones/Struck Idiophones/Claps" \
  "Idiophones/Struck Idiophones/Shaker, Small"

# Salas de Voxengo (uso comercial libre; no se redistribuyen). Se usa «Nice Drum Room».
mkdir -p muestras/ir
curl -L -o muestras/ir/IMreverbs.zip https://www.voxengo.com/files/impulses/IMreverbs.zip
unzip muestras/ir/IMreverbs.zip -d muestras/ir

npm run audio       # public/audio/mix-{reel,wide,estado}.m4a
```

Si las muestras están en otra carpeta, usa `VCSL=<carpeta> python3 -I scripts/audio/musica.py <carpeta de las
salas> --format reel`.

## Créditos y licencias

- Código: MIT, como el resto del repositorio.
- Fuentes: Inter y JetBrains Mono, SIL Open Font License 1.1. Las licencias están en `public/fonts/`.
- Muestras: VCSL (Versilian Community Sample Library), CC0 1.0.
- Salas: Voxengo Free Reverb Impulse Responses, uso comercial libre y sin redistribución.
- Groove: `scripts/audio/motor/groove.json` deriva del
  [Groove MIDI Dataset](https://magenta.withgoogle.com/datasets/groove) de Gillick, Roberts, Engel, Eck y Bamman
  (Google Magenta), CC BY 4.0.
- El logotipo (`public/marca/`) es la marca de thealejandro y no está cubierto por la licencia MIT.
- Screensaver FX se inspira en Omarchy y en terminaltexteffects. No tiene relación con Microsoft ni con Apple.
