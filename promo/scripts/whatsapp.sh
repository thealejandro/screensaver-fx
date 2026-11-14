#!/usr/bin/env bash
# Perfil WhatsApp (estados): 30 fps, CRF 20 con tope de 8 Mb/s (< 25 MB para el móvil), AAC 128 kb/s, +faststart.
set -euo pipefail
in="$1"; out="$2"
ffmpeg -loglevel error -y -i "$in" -vf fps=30 -c:v libx264 -profile:v high -preset slow \
  -crf 20 -maxrate 8M -bufsize 16M -pix_fmt yuv420p -c:a aac -b:a 128k -movflags +faststart "$out"
ffprobe -v error -show_entries stream=codec_name,width,height,r_frame_rate,bit_rate -show_entries format=duration,size -of compact "$out"
