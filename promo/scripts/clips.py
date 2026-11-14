#!/usr/bin/env python3
"""Clips del motor real para el vídeo: core/examples/render.rs → ffmpeg (escalado `neighbor`) → public/clips/.

    python3 -I scripts/clips.py [--solo <prefijo>] [--forzar]

Compila el ejemplo `render` (cargo, --release) y, para cada clip de CLIPS, le pide al motor los
fotogramas de un tramo de su tiempo (el primer efecto empieza a los 0,15 s; `effects=<id>` fuerza uno)
como RGB en bruto. El búfer del motor se amplía con un factor entero (vecino más cercano, píxel nítido)
y se centra en la pantalla de la pieza. Determinista: misma semilla, misma hora de pared.

Salida: public/clips/<orientación>/<id>.mp4 (H.264 CRF 12, 60 fps, sin audio). No se versionan.
"""
from __future__ import annotations

import argparse
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PROMO = os.path.dirname(HERE)
REPO = os.path.dirname(PROMO)
OUT = os.path.join(PROMO, 'public', 'clips')
TARGET = os.environ.get('CARGO_TARGET_DIR') or os.path.join(REPO, 'target')
RENDER = os.path.join(TARGET, 'release', 'examples', 'render')
FPS = 60

# Pantalla simulada (ancho, alto del motor) y tamaño final del clip.
PANTALLA = {'v': (1080, 1920, 1080, 1920), 'h': (1920, 1080, 1920, 1080), 't': (1920, 1080, 480, 270)}
# Rótulo de los clips: en vertical solo «FX» (escala x3 exacta); en horizontal, «FX» y «screensaver».
ROTULO = {'v': 'banner=text|title=FX', 'h': 'banner=text|title=FX|subtitle=screensaver', 't': 'banner=text|title=FX|subtitle=screensaver'}
TEXTO = {'es': 'banner=text|title=CAFÉ|subtitle=ya vuelvo', 'en': 'banner=text|title=BRB|subtitle=back soon'}
ARTE = 'banner=file|art=' + os.path.join(PROMO, 'public', 'arte.txt')
ARTE_FX = 'neon'
COMUN = 'seed=7|now=2026-11-09 21:47:10|weekday=0'
TEMA = 'aero-night'

# Segundo del motor en el que cada efecto ya se mueve de forma reconocible (revisado en tiras de fotogramas).
INICIO = {
    'neon': 0.6, 'starfield': 2.0, 'terminal': 2.2, 'glitch': 1.3, 'plasma': 1.5, 'clock': 0.6,
    'decrypt': 0.5, 'rain': 0.6, 'beams': 0.6, 'slide': 0.4, 'scatter': 1.0, 'burn': 1.5,
    'matrix': 1.2, 'waves': 1.0, 'spotlights': 1.5, 'wipe': 0.9,
}
EFECTOS = list(INICIO)
TEMAS_VERTICAL = ['aero-night', 'obsidiana', 'ventura', 'big-sur', 'fluent', 'cupertino']
TEMAS = ['aero-night', 'obsidiana', 'ventura', 'big-sur', 'fluent', 'cupertino', 'graphite', 'tahoe',
         'aero', 'fluent-light', 'cupertino-light']


def clips() -> list[dict]:
    out = []

    def add(cid, o, fx, desde, dura, tema=TEMA, extra='', ancho=0):
        out.append(dict(id=cid, o=o, fx=fx, desde=desde, dura=dura, tema=tema, extra=extra, ancho=ancho))

    for lang in ('es', 'en'):
        # El descifrado termina de resolver el texto hacia los 5 s del motor. En vertical el rótulo ocupa
        # todo el ancho: se deja en 920 px (márgenes laterales del 7 %).
        add(f'v/texto-{lang}', 'v', 'decrypt', 3.8, 2.2, extra=TEXTO[lang] + f'|lang={lang}', ancho=920)
        add(f'h/texto-{lang}', 'h', 'decrypt', 3.4, 3.0, extra=TEXTO[lang] + f'|lang={lang}')
        add(f'v/fx-clock-{lang}', 'v', 'clock', INICIO['clock'], 1.4, extra=f'lang={lang}')
        add(f'h/fx-clock-{lang}', 'h', 'clock', INICIO['clock'], 1.0, extra=f'lang={lang}')
    # Arte ASCII propio (public/arte.txt hace de screensaver.txt).
    add('v/arte', 'v', ARTE_FX, INICIO[ARTE_FX], 1.8, extra=ARTE)
    add('h/arte', 'h', ARTE_FX, INICIO[ARTE_FX], 2.8, extra=ARTE)
    add('v/gancho', 'v', 'scatter', 1.0, 3.6)
    add('h/gancho', 'h', 'scatter', 0.9, 5.2)
    for fx in EFECTOS:
        if fx != 'clock':
            add(f'v/fx-{fx}', 'v', fx, INICIO[fx], 1.4)
            add(f'h/fx-{fx}', 'h', fx, INICIO[fx], 1.0)
    # Temas: el mismo plasma sigue su curso y cambia de tema en cada corte (2 pulsos = 0,8 s).
    for i, tema in enumerate(TEMAS_VERTICAL):
        add(f'v/tema-{tema}', 'v', 'plasma', 3.0 + 0.8 * i, 1.1, tema=tema)
    for tema in TEMAS + ['auto']:
        add(f't/tema-{tema}', 't', 'plasma', 3.0, 6.6, tema=tema)
    add('v/fondo', 'v', 'starfield', 3.0, 3.4)
    add('h/fondo', 'h', 'starfield', 3.0, 8.2)
    return out


def info(w: int, h: int, ini: str) -> list[int]:
    r = subprocess.run([RENDER, str(w), str(h), str(FPS), '0', '0', ini, '--info'], capture_output=True, text=True, check=True)
    return [int(x) for x in r.stdout.split()]


def render(c: dict, forzar: bool) -> None:
    path = os.path.join(OUT, c['id'] + '.mp4')
    if os.path.exists(path) and not forzar:
        return
    os.makedirs(os.path.dirname(path), exist_ok=True)
    w, h, ow, oh = PANTALLA[c['o']]
    base = TEXTO_O_ROTULO(c)
    ini = f"{COMUN}|theme={c['tema']}|effects={c['fx']}|{base}"
    if c['tema'] == 'auto':
        ini += '|accent=0078d4'
    bw, bh, _dx, _dy, dw, _dh, _cols, _rows = info(w, h, ini)
    # Factor entero: el más cercano al del motor si el recorte que provoca cabe en el margen del rótulo.
    k = max(1, round(dw / bw))
    if bw * k - w > 64:
        k = max(1, dw // bw)
    vf = [f'scale={bw * k}:{bh * k}:flags=neighbor']
    sw, sh = bw * k, bh * k
    if c.get('ancho') and sw > c['ancho']:
        # El rótulo llena el ancho de la pantalla: se reduce (filtro de área) para dejar el margen seguro.
        sh = round(sh * c['ancho'] / sw / 2) * 2
        sw = c['ancho']
        vf.append(f'scale={sw}:{sh}:flags=area')
    if sw > w or sh > h:
        vf.append(f'crop={min(sw, w)}:{min(sh, h)}')
        sw, sh = min(sw, w), min(sh, h)
    vf.append(f'pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:black')
    if (ow, oh) != (w, h):
        vf.append(f'scale={ow}:{oh}:flags=area')
    hasta = c['desde'] + c['dura']
    cmd = [RENDER, str(w), str(h), str(FPS), f"{c['desde']:.4f}", f'{hasta:.4f}', ini]
    enc = ['ffmpeg', '-v', 'error', '-y', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-s', f'{bw}x{bh}', '-r', str(FPS),
           '-i', '-', '-vf', ','.join(vf), '-c:v', 'libx264', '-preset', 'medium', '-crf', '12', '-g', '30',
           '-pix_fmt', 'yuv420p', '-movflags', '+faststart', path]
    src = subprocess.Popen(cmd, stdout=subprocess.PIPE)
    subprocess.run(enc, stdin=src.stdout, check=True)
    src.stdout.close()
    if src.wait() != 0:
        sys.exit(f'render falló en {c["id"]}')
    print(f"{c['id']}: {c['fx']} {c['tema']} t={c['desde']}–{hasta:.2f} s, búfer {bw}x{bh} x{k}", flush=True)


def TEXTO_O_ROTULO(c: dict) -> str:
    """El rótulo de la orientación, salvo en los clips de texto propio (ya lo traen en `extra`)."""
    if 'banner=' in c['extra']:
        return c['extra']
    return ROTULO[c['o']] + (f"|{c['extra']}" if c['extra'] else '')


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument('--solo', default='', help='solo los clips cuyo id empiece así (p. ej. v/ o h/fx-)')
    ap.add_argument('--forzar', action='store_true', help='rehacer aunque ya existan')
    a = ap.parse_args()
    subprocess.run(['cargo', 'build', '--manifest-path', os.path.join(REPO, 'Cargo.toml'), '-p', 'ssfx-core',
                    '--example', 'render', '--release', '--quiet'], check=True)
    for c in clips():
        if c['id'].startswith(a.solo):
            render(c, a.forzar)


if __name__ == '__main__':
    main()
