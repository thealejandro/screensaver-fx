#!/usr/bin/env bash
# Versión web del reel: 720x1280 (vertical), H.264 CRF 23 con tope de tasa, AAC 128 kb/s, +faststart.
# Falla si el archivo pasa de 8 MB.
#
#   scripts/web.sh <entrada.mp4> <salida.mp4>
set -euo pipefail
in="$1"; out="$2"
max=$((8 * 1024 * 1024))
mkdir -p "$(dirname "$out")"
ffmpeg -loglevel error -y -i "$in" -vf scale=720:-2:flags=lanczos,fps=30 -c:v libx264 -profile:v high -preset slow \
  -crf 23 -maxrate 1.9M -bufsize 3.8M -pix_fmt yuv420p -c:a aac -b:a 128k -movflags +faststart "$out"
size=$(stat -c %s "$out")
ffprobe -v error -show_entries stream=codec_name,width,height,r_frame_rate -show_entries format=duration,size -of compact "$out"
if [ "$size" -gt "$max" ]; then
  echo "ERROR: $out pesa $size bytes (> 8 MB)" >&2
  exit 1
fi
