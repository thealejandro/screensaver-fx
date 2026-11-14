"""Instrumentos grabados de verdad (VCSL, CC0) como sampler sencillo.

Cada instrumento indexa sus WAV por nota (sacada del nombre del archivo) y toca cualquier nota MIDI con la
muestra más cercana, re-muestreada (±3 semitonos como mucho para que no suene a ardilla).
"""
from __future__ import annotations

import glob
import os
import re
import subprocess

import numpy as np

SR = 48000
# Muestras de VCSL fuera del repo (ver promo/README.md): $VCSL o promo/muestras/vcsl.
ROOT = os.environ.get('VCSL') or os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', 'muestras', 'vcsl')
NOTES = {'C': 0, 'C#': 1, 'D': 2, 'D#': 3, 'E': 4, 'F': 5, 'F#': 6, 'G': 7, 'G#': 8, 'A': 9, 'A#': 10, 'B': 11}
_cache: dict[str, np.ndarray] = {}


def load(path: str) -> np.ndarray:
    if path not in _cache:
        raw = subprocess.run(['ffmpeg', '-v', 'error', '-i', path, '-ac', '1', '-ar', str(SR), '-f', 'f32le', '-'],
                             capture_output=True, check=True).stdout
        x = np.frombuffer(raw, np.float32).astype(np.float64)
        # quita el silencio inicial (hasta el ataque) para que caiga a tiempo
        on = np.argmax(np.abs(x) > 0.02 * np.abs(x).max())
        _cache[path] = x[max(0, on - 48):]
    return _cache[path]


def midi_of(name: str) -> int | None:
    m = re.search(r'(?:^|[_\s])([A-G]#?)(-?\d)(?=[_.\s])', name)
    if not m:
        return None
    return 12 * (int(m.group(2)) + 1) + NOTES[m.group(1)]


class Sampler:
    def __init__(self, folder: str, prefer: str = '', avoid: str = '$^'):
        files = [f for f in glob.glob(os.path.join(ROOT, folder, '**', '*.wav'), recursive=True)
                 if not re.search(avoid, os.path.basename(f))]
        self.map: dict[int, list[str]] = {}
        for f in files:
            n = midi_of(os.path.basename(f))
            if n is not None:
                self.map.setdefault(n, []).append(f)
        for n, fs in self.map.items():
            good = [f for f in fs if re.search(prefer, os.path.basename(f))] if prefer else []
            self.map[n] = sorted(good or fs)
        assert self.map, folder
        self.rr = 0

    def __call__(self, midi: float, dur: float = 2.0, fade: float = 0.25) -> np.ndarray:
        keys = np.array(sorted(self.map))
        k = int(keys[np.argmin(np.abs(keys - midi))])
        fs = self.map[k]
        self.rr += 1
        x = load(fs[self.rr % len(fs)])
        ratio = 2 ** ((midi - k) / 12)
        if abs(ratio - 1) > 1e-6:
            idx = np.arange(0, len(x) - 1, ratio)
            x = np.interp(idx, np.arange(len(x)), x)
        n = min(len(x), int(dur * SR))
        y = x[:n].copy()
        f = min(int(fade * SR), n)
        y[n - f:] *= np.linspace(1, 0, f) ** 2
        return y / (np.abs(y).max() + 1e-9)


class VelSampler:
    """Sampler con capas de fuerza: `layers` de la más suave a la más fuerte (texto que aparece en el nombre).

    Devuelve la muestra con su nivel natural escalado por la velocidad (0–1), sin normalizar cada nota, y
    deja sonar la cola hasta `dur` (el pedal o la mano la apagan con un fundido corto, `fade`).
    """

    def __init__(self, folder: str, layers: list[str], avoid: str = '$^'):
        self.layers = [Sampler(folder, prefer=l, avoid=avoid) for l in layers]
        for s, l in zip(self.layers, layers):  # quita las notas que no tienen esa capa (prefer cae a todas)
            s.map = {n: [f for f in fs if l in os.path.basename(f)] for n, fs in s.map.items()}
            s.map = {n: fs for n, fs in s.map.items() if fs}
        self.peak = max(np.abs(load(fs[0])).max() for fs in self.layers[-1].map.values())

    def __call__(self, midi: float, vel: float, dur: float = 4.0, fade: float = 0.12) -> np.ndarray:
        i = min(len(self.layers) - 1, int(vel * len(self.layers)))
        s = self.layers[i]
        keys = np.array(sorted(s.map))
        k = int(keys[np.argmin(np.abs(keys - midi))])
        s.rr += 1
        x = load(s.map[k][s.rr % len(s.map[k])])
        ratio = 2 ** ((midi - k) / 12)
        if abs(ratio - 1) > 1e-6:
            x = np.interp(np.arange(0, len(x) - 1, ratio), np.arange(len(x)), x)
        n = min(len(x), int(dur * SR))
        y = x[:n] / self.peak
        f = min(int(fade * SR), n)
        y[n - f:] *= np.linspace(1, 0, f) ** 2
        # dentro de la capa, la velocidad aún matiza un poco el nivel
        lo, hi = i / len(self.layers), (i + 1) / len(self.layers)
        return y * (0.75 + 0.25 * (vel - lo) / (hi - lo))


class Hits:
    """Percusión sin altura: elige por nombre (p. ej. 'hit1_f') y alterna round-robin."""

    def __init__(self, folder: str):
        self.files = sorted(glob.glob(os.path.join(ROOT, folder, '**', '*.wav'), recursive=True))
        self.rr = 0

    def __call__(self, pat: str = '', dur: float = 1.0) -> np.ndarray:
        fs = [f for f in self.files if re.search(pat, os.path.basename(f))] or self.files
        self.rr += 1
        x = load(fs[self.rr % len(fs)])[:int(dur * SR)].copy()
        f = min(int(0.03 * SR), len(x))
        x[len(x) - f:] *= np.linspace(1, 0, f)
        return x / (np.abs(x).max() + 1e-9)


I = 'Idiophones/Struck Idiophones/'
INSTR = {
    'kalimba-kenia': lambda: Sampler('Idiophones/Plucked Idiophones/Kalimba, Kenya'),
    'kalimba-tanzania': lambda: Sampler('Idiophones/Plucked Idiophones/Kalimba, Tanzania'),
    'mbira': lambda: Sampler('Idiophones/Plucked Idiophones/Mbira dzaVadzimu Nyamaropa, Zimbabwe, Low B'),
    'balafon': lambda: Sampler(I + 'Balafon/Soft Mallet', 'vl2'),
    'vibrafono': lambda: Sampler(I + 'Vibraphone/Soft Mallets', 'v1'),
    'marimba': lambda: Sampler(I + 'Marimba', 'soft'),
    'campanas-mano': lambda: Sampler(I + 'Hand Chimes'),
    'piano-vertical': lambda: Sampler('Chordophones/Zithers/Upright Piano, Knight/Sustains', 'vl1'),
    'arpa-folk': lambda: Sampler('Chordophones/Composite Chordophones/Folk Harp', 'v2'),
    'strumstick': lambda: Sampler('Chordophones/Composite Chordophones/Strumstick/Finger', 'vl2'),
    'dan-tranh': lambda: Sampler('Chordophones/Zithers/Dan Tranh/Normal', 'mf'),
}
