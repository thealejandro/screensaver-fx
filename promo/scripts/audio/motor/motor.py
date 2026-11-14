"""Motor de música: muestras grabadas (VCSL), groove humano, dinámica, sala real y máster.

Copiado del motor propio de thealejandro/promos (MIT), sin el motivo ni los acordes de aquella marca:
la música de este vídeo (scripts/audio/musica.py) tiene los suyos.
"""
import json
import os
import subprocess
import sys
import wave

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from instrumentos import onepole_lp, place, wood  # noqa: E402,F401
from muestras import SR, Hits, VelSampler, load  # noqa: E402,F401

GROOVE = json.load(open(os.path.join(HERE, 'groove.json')))  # Groove MIDI Dataset (Magenta), CC BY 4.0
MEAN_OFF = np.mean([x['off'] for p in ('bombo', 'caja', 'charles') for x in GROOVE[p] if x['n']])
I = 'Idiophones/Struck Idiophones/'


def write(path, bus, lufs=-18):
    """Máster: compresión suave, agudos recortados (que no «chille»), algo de cuerpo y nivel final."""
    raw = path[:-4] + '-raw.wav'
    x = np.clip(bus / (np.abs(bus).max() + 1e-9) * 0.8, -1, 1)
    with wave.open(raw, 'wb') as w:
        w.setnchannels(2)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes((x * 32767).astype('<i2').tobytes())
    af = ('acompressor=threshold=0.12:ratio=2.2:attack=20:release=300:knee=4,highshelf=f=5500:g=-4,'
          f'lowpass=f=13000,equalizer=f=220:t=q:w=1:g=1.5,loudnorm=I={lufs}:TP=-1.5:LRA=11')
    subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', raw, '-af', af, '-ar', '48000', path], check=True)
    os.remove(raw)


def room(bus, ir_path, wet_db, predelay=0.012):
    """Convolución con una sala real (respuesta al impulso); el envío no lleva graves (no embarra)."""
    ir = load(ir_path)  # mono a 48 kHz
    ir = ir / np.sqrt(np.sum(ir ** 2))
    ir = np.concatenate([np.zeros(int(predelay * SR)), ir])
    n = len(bus) + len(ir)
    N = 1 << (n - 1).bit_length()
    H = np.fft.rfft(ir, N)
    wet = np.stack([np.fft.irfft(np.fft.rfft(bus[:, c], N) * H, N)[:len(bus)] for c in range(2)], 1)
    # paso alto suave del envío (~250 Hz): se resta un paso bajo de un polo
    a = np.exp(-2 * np.pi * 250 / SR)
    lo = np.empty_like(wet)
    s = np.zeros(2)
    for i in range(len(wet)):  # bucle simple; 30 s a 48 kHz tarda unos segundos
        s = (1 - a) * wet[i] + a * s
        lo[i] = s
    wet = wet - lo
    # ensancha un poco: un canal del envío llega 7 ms tarde
    d = int(0.007 * SR)
    wet[d:, 1] = wet[:-d, 1].copy()
    g = 10 ** (wet_db / 20) * np.abs(bus).max() / (np.abs(wet).max() + 1e-9)
    return bus + wet * g


class Feel:
    """Tiempo y fuerza «humanos»: plantilla de groove por semicorchea + arco de frase + ruido correlado.

    `arch` = (posiciones en compases, niveles) del arco de la frase.
    """

    def __init__(self, bpm, seed, off=0.4, arch=((0, 1, 2, 3.2, 4.6, 5, 6), (0.78, 0.84, 0.92, 1.0, 0.82, 0.86, 0.8))):
        self.beat = 60 / bpm
        self.off = off
        self.arch = arch
        self.rng = np.random.default_rng(seed)
        self.drift = 0.0

    def t(self, bar, b, part='charles', amount=1.0, lag=0.0):
        g = GROOVE[part][int(round(b * 4)) % 16]
        off = (g['off'] - MEAN_OFF) * amount + self.rng.normal(0, g['sd'] * 0.25 * amount)
        return self.off + (bar * 4 + b + off) * self.beat + lag

    def v(self, bar, b, part='charles', base=0.6, swing=0.35):
        arch = np.interp(bar + b / 4, *self.arch)
        g = GROOVE[part][int(round(b * 4)) % 16]['vel']
        ref = max(x['vel'] for x in GROOVE[part])
        accent = 1 - swing + swing * g / ref
        self.drift = 0.8 * self.drift + self.rng.normal(0, 0.035)  # la mano «va y viene»
        return float(np.clip(base * arch * accent * (1 + self.drift), 0.05, 1))


def roll(n_voices, spread):
    """Retardos de un acorde tocado a mano (de grave a agudo), con algo de irregularidad."""
    return np.cumsum([0] + list(np.random.default_rng(n_voices).uniform(0.6, 1.4, n_voices - 1) * spread))
