"""Parte genérica del sonido: línea de tiempo por formato, efectos a partir de cues, máster y salida.

Adaptado de thealejandro/promos (scripts/audio/reel.py, MIT). La música y los cues están en musica.py:

    from mezcla import run, starts, load_timeline
    run(music, cues, PEAK)        # lee argumentos: irs [--format reel] [--stems DIR] [--out FILE]

src/timeline.json tiene una clave por formato (reel, wide, estado); el audio lee el mismo archivo que el
vídeo. Salida por defecto: public/audio/mix-<formato>.m4a. Nivel: −15 LUFS reel y wide, −18 LUFS estado,
pico real ≤ −1 dBTP medido sobre el AAC final.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import tempfile
from typing import Callable

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, 'motor'))
from motor import write  # noqa: E402
from promolib import FPS, SR, db, ebur128, finish, master, save, sfx_sub, sfx_tick, sfx_whoosh  # noqa: E402
from promolib import load as pload  # noqa: E402

ROOT = os.path.abspath(os.path.join(HERE, '..', '..'))
FORMATOS = ('reel', 'wide', 'estado')
LUFS = {'reel': -15.0, 'wide': -15.0, 'estado': -18.0}


def load_timeline() -> dict:
    with open(os.path.join(ROOT, 'src', 'timeline.json')) as fh:
        return json.load(fh)


def starts(tl: dict, formato: str) -> dict[str, int]:
    """Fotograma en el que empieza cada escena del formato (y `_end`)."""
    out, t = {}, 0
    for e in tl[formato]:
        out[e['id']] = t
        t += e['beats'] * tl['beat']
    out['_end'] = t
    return out


def sec(frames: float) -> float:
    return frames / FPS


def sfx_bus(dur: float, cues: list[dict], peak: dict[str, float]) -> np.ndarray:
    """Efectos sintetizados colocados en cada cue (`kind`: tick|whoosh|sub, con sufijo _s para el nivel suave)."""
    n = int(round(dur * SR))
    bus = np.zeros((n, 2))
    for idx, c in enumerate(cues):
        rng = np.random.default_rng(2000 + idx)
        base = c['kind'].removesuffix('_s')
        if base == 'tick':
            y, on = sfx_tick(rng, c.get('pitch', 1), c.get('bright', 1), c.get('p', 0))
        elif base == 'whoosh':
            y, on = sfx_whoosh(rng, c.get('dur', 0.8), 0.55, c.get('lo', 300), c.get('hi', 2600), c.get('p0', -0.5), c.get('p1', 0.5))
        elif base == 'sub':
            y, on = sfx_sub(rng, c.get('f0', 68), c.get('f1', 38), c.get('decay', 0.42))
        else:
            raise ValueError(c['kind'])
        jitter = rng.uniform(-1.2, 1.2) if base == 'tick' else 0
        y = finish(y, peak[c['kind']] + jitter)
        p = int(round(c['t'] * SR)) - on
        a, b = max(0, p), min(n, p + len(y))
        if b > a:
            bus[a:b] += y[a - p:b - p]
    return bus


def run(
    music: Callable[[str, str], np.ndarray],
    cues: Callable[[str], list[dict]],
    peak: dict[str, float],
    argv: list[str] | None = None,
) -> None:
    """Mezcla música + efectos, masteriza y escribe el m4a."""
    ap = argparse.ArgumentParser()
    ap.add_argument('irs', help='carpeta con las respuestas al impulso de Voxengo')
    ap.add_argument('--format', default='reel', choices=FORMATOS, dest='formato')
    ap.add_argument('--stems', help='carpeta para música, efectos y tabla de eventos por separado')
    ap.add_argument('--out', help='salida (por defecto public/audio/mix-<formato>.m4a)')
    a = ap.parse_args(argv)
    f = a.formato
    tl = load_timeline()
    dur = sec(starts(tl, f)['_end'])
    out = a.out or os.path.join(ROOT, 'public', 'audio', f'mix-{f}.m4a')
    with tempfile.TemporaryDirectory() as tmp:
        print('música…', flush=True)
        write(os.path.join(tmp, 'mus.wav'), music(a.irs, f), lufs=-17)
        mus = pload(os.path.join(tmp, 'mus.wav'))
        mus *= db(-17 - ebur128(os.path.join(tmp, 'mus.wav'))['I'])
        print('efectos…', flush=True)
        C = sorted(cues(f), key=lambda x: x['t'])
        fx = sfx_bus(dur, C, peak)
        n = min(len(mus), len(fx))
        mix = mus[:n] + fx[:n]
        t = np.arange(len(mix)) / SR
        mix *= np.clip((dur - t) / 0.25, 0, 1)[:, None]
        os.makedirs(os.path.dirname(out), exist_ok=True)
        m = master(mix, out, tmp, target=LUFS[f])
        print(f'{out}: I={m["I"]} LUFS, LRA={m["LRA"]} LU, pico real={m["TP"]} dBTP')
        if a.stems:
            os.makedirs(a.stems, exist_ok=True)
            save(os.path.join(a.stems, 'music.wav'), mus)
            save(os.path.join(a.stems, 'sfx.wav'), fx)
            with open(os.path.join(a.stems, 'cues.json'), 'w') as fh:
                json.dump(dict(cues=C, dur=dur), fh, ensure_ascii=False, indent=1)
