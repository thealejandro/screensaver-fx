#!/usr/bin/env bash
# Informe de calidad de unos MP4: ffprobe, sonoridad (ebur128: I, LRA, pico real) y un mosaico de
# fotogramas (uno cada 2 s, o más espaciados si el vídeo es largo; 5x3) por archivo.
#
#   scripts/qa.sh <carpeta-de-salida> <archivo.mp4>...      → informe.txt y <archivo>-mosaico.jpg
set -euo pipefail
dir="$1"; shift
mkdir -p "$dir"
report="$dir/informe.txt"
: > "$report"
for f in "$@"; do
  name="$(basename "$f" .mp4)"
  dur="$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$f")"
  size="$(stat -c %s "$f")"
  step="$(awk -v d="$dur" 'BEGIN { s = int((d + 14) / 15); if (s < 2) s = 2; print s }')"
  ffmpeg -loglevel error -y -i "$f" -vf "fps=1/$step,scale=w='if(gt(iw\,ih)\,384\,216)':h=-2,tile=5x3" -frames:v 1 -q:v 3 "$dir/$name-mosaico.jpg"
  loud="$(ffmpeg -nostats -hide_banner -i "$f" -af ebur128=peak=true -f null - 2>&1 | awk '/Summary:/ {s=1} s && /^ *I:/ {i=$2} s && /^ *LRA:/ {l=$2} s && /Peak:/ {p=$2} END {print "I=" i " LUFS, LRA=" l " LU, pico real=" p " dBTP"}')"
  {
    echo "== $f"
    ffprobe -v error -show_entries stream=codec_name,pix_fmt,width,height,r_frame_rate -show_entries format=duration -of compact "$f"
    awk -v s="$size" 'BEGIN { printf "tamaño=%.2f MB\n", s / 1048576 }'
    echo "$loud"
    echo "mosaico=$dir/$name-mosaico.jpg (un fotograma cada ${step} s)"
    echo
  } | tee -a "$report"
done
