#!/usr/bin/env bash
# Remuxa la salida de Remotion con +faststart (sin recodificar) y borra el intermedio.
set -euo pipefail
in="$1"; out="$2"
ffmpeg -loglevel error -y -i "$in" -c copy -movflags +faststart "$out"
rm -f "$in"
ffprobe -v error -show_entries stream=codec_name,pix_fmt,width,height,r_frame_rate -show_entries format=duration -of compact "$out"
