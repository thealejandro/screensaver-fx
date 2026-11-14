#!/usr/bin/env bash
# Genera todas las piezas: vídeos (remotion → finish.sh), versión WhatsApp (reel y estado), versión web 720p
# del reel, portada y póster, el clip corto del README (inglés) y el informe de calidad en out/qa/.
#
#   scripts/build.sh [--lang es,en] [--formats reel,wide,estado]
#
# Antes: npm run clips (public/clips) y la música (public/audio/mix-<formato>.m4a, ya versionada).
#
# Salidas:
#   out/screensaver-fx-<formato>-<lang>.mp4            1080x1920 (reel, estado) o 1920x1080 (wide), 60 fps
#   out/screensaver-fx-<formato>-<lang>-whatsapp.mp4   30 fps, < 25 MB (reel y estado)
#   out/web/screensaver-fx-720p[-en].mp4               720x1280, <= 8 MB (reel)
#   out/web/screensaver-fx-poster[-en].jpg             póster a 720 px de ancho
#   out/screensaver-fx-cover-<lang>.jpg y -poster-<lang>.jpg
#   out/readme/screensaver-fx-en.mp4 y .gif            primeros 12 s del horizontal en inglés, sin audio
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

langs="es,en"
formats="reel,wide,estado"
while [ $# -gt 0 ]; do
  case "$1" in
    --lang) langs="$2"; shift 2 ;;
    --formats) formats="$2"; shift 2 ;;
    *) echo "argumento desconocido: $1" >&2; exit 2 ;;
  esac
done

[ -d public/clips/v ] && [ -d public/clips/h ] && [ -d public/clips/t ] || { echo "faltan los clips: npm run clips" >&2; exit 1; }

slug=screensaver-fx
out=out
mkdir -p "$out/web" "$out/readme"
made=()
max_wa=$((25 * 1000 * 1000))
nombre() { case "$1" in reel) echo Reel ;; wide) echo Wide ;; estado) echo Estado ;; esac; }

for lang in ${langs//,/ }; do
  for fmt in ${formats//,/ }; do
    id="Promo-$(nombre "$fmt")-$lang"
    base="$out/$slug-$fmt-$lang"
    echo "== $id"
    npx remotion render src/index.ts "$id" "$base.raw.mp4"
    bash scripts/finish.sh "$base.raw.mp4" "$base.mp4"
    made+=("$base.mp4")
    if [ "$fmt" = "reel" ] || [ "$fmt" = "estado" ]; then
      bash scripts/whatsapp.sh "$base.mp4" "$base-whatsapp.mp4"
      [ "$(stat -c %s "$base-whatsapp.mp4")" -lt "$max_wa" ] || { echo "ERROR: $base-whatsapp.mp4 no baja de 25 MB" >&2; exit 1; }
      made+=("$base-whatsapp.mp4")
    fi
    suffix=""; [ "$lang" = "es" ] || suffix="-$lang"
    if [ "$fmt" = "reel" ]; then
      bash scripts/web.sh "$base.mp4" "$out/web/$slug-720p$suffix.mp4"
      made+=("$out/web/$slug-720p$suffix.mp4")
    fi
    if [ "$fmt" = "wide" ] && [ "$lang" = "en" ]; then
      # README: los 12 primeros segundos (gancho y clásicos), 960x540, sin audio; y un GIF de 480 px.
      ffmpeg -loglevel error -y -i "$base.mp4" -t 12 -an -vf scale=960:-2:flags=lanczos,fps=30 -c:v libx264 \
        -preset slow -crf 24 -pix_fmt yuv420p -movflags +faststart "$out/readme/$slug-en.mp4"
      ffmpeg -loglevel error -y -i "$base.mp4" -t 12 \
        -vf "fps=12,scale=480:-2:flags=lanczos,split[a][b];[a]palettegen=max_colors=96:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4" \
        "$out/readme/$slug-en.gif"
    fi
  done
  for pieza in cover poster; do
    case "$pieza" in cover) id="Cover-$lang" ;; poster) id="Poster-$lang" ;; esac
    echo "== $id"
    npx remotion still src/index.ts "$id" "$out/$slug-$pieza-$lang.png"
    ffmpeg -loglevel error -y -i "$out/$slug-$pieza-$lang.png" -q:v 2 "$out/$slug-$pieza-$lang.jpg"
    rm -f "$out/$slug-$pieza-$lang.png"
    if [ "$pieza" = "poster" ]; then
      suffix=""; [ "$lang" = "es" ] || suffix="-$lang"
      ffmpeg -loglevel error -y -i "$out/$slug-poster-$lang.jpg" -vf scale=720:-2 -q:v 3 "$out/web/$slug-poster$suffix.jpg"
    fi
  done
done

bash scripts/qa.sh "$out/qa" "${made[@]}"
