"""Instrumentos sintetizados propios (numpy), secos y de ataque: nada de pads ni drones.

Cada instrumento devuelve un array mono float64 a SR. Todo es determinista con `seed`.
"""
from __future__ import annotations

import numpy as np

SR = 48000


def hz(midi: float) -> float:
    return 440.0 * 2 ** ((midi - 69) / 12)


def env_exp(n: int, decay: float, attack: float = 0.002) -> np.ndarray:
    t = np.arange(n) / SR
    a = np.clip(t / max(attack, 1e-4), 0, 1)
    return a * np.exp(-t / decay)


def onepole_lp(x: np.ndarray, fc: float) -> np.ndarray:
    a = np.exp(-2 * np.pi * fc / SR)
    y = np.empty_like(x)
    s = 0.0
    for i, v in enumerate(x):
        s = (1 - a) * v + a * s
        y[i] = s
    return y


def pluck(midi: float, dur: float = 0.6, bright: float = 0.6, seed: int = 0) -> np.ndarray:
    """Cuerda pulsada (Karplus-Strong) con amortiguación: suena a pizzicato/guitarra apagada."""
    rng = np.random.default_rng(seed)
    f = hz(midi)
    p = int(round(SR / f))
    n = int(dur * SR)
    buf = rng.uniform(-1, 1, p)
    buf = onepole_lp(buf, 1500 + 7000 * bright)
    y = np.empty(n)
    damp = 0.996 - 0.01 * (1 - bright)
    for i in range(n):
        v = buf[i % p]
        y[i] = v
        buf[i % p] = damp * 0.5 * (v + buf[(i + 1) % p])
    return y * env_exp(n, dur * 0.45, 0.001)


def mallet(midi: float, dur: float = 0.9, hard: float = 0.5, seed: int = 0) -> np.ndarray:
    """Marimba/kalimba modal: parciales inarmónicos que decaen rápido (ataque claro, sin cola larga)."""
    f = hz(midi)
    n = int(dur * SR)
    t = np.arange(n) / SR
    y = np.zeros(n)
    for ratio, amp, dec in [(1, 1.0, 0.45), (3.93, 0.35 * hard, 0.09), (9.4, 0.12 * hard, 0.03)]:
        if f * ratio < SR / 2.2:
            y += amp * np.sin(2 * np.pi * f * ratio * t) * np.exp(-t / (dec * dur / 0.9))
    click = np.random.default_rng(seed).normal(0, 1, n) * np.exp(-t / 0.002) * 0.15 * hard
    return (y + click) * np.clip(t / 0.001, 0, 1)


def fm_key(midi: float, dur: float = 0.5, index: float = 2.0) -> np.ndarray:
    """Tecla FM corta (tipo piano eléctrico apagado)."""
    f = hz(midi)
    n = int(dur * SR)
    t = np.arange(n) / SR
    mod = index * np.exp(-t / 0.08) * np.sin(2 * np.pi * f * t)
    return np.sin(2 * np.pi * f * t + mod) * env_exp(n, dur * 0.35, 0.003)


def bass(midi: float, dur: float = 0.35) -> np.ndarray:
    """Bajo corto: seno + 2.º armónico con caída, sin sostener."""
    f = hz(midi)
    n = int(dur * SR)
    t = np.arange(n) / SR
    y = np.sin(2 * np.pi * f * t) + 0.3 * np.sin(4 * np.pi * f * t) * np.exp(-t / 0.05)
    e = env_exp(n, dur * 0.5, 0.004)
    e[-int(0.01 * SR):] *= np.linspace(1, 0, int(0.01 * SR))
    return np.tanh(1.4 * y) * e


def kick(dur: float = 0.3, f0: float = 150, f1: float = 48) -> np.ndarray:
    n = int(dur * SR)
    t = np.arange(n) / SR
    f = f1 + (f0 - f1) * np.exp(-t / 0.03)
    ph = 2 * np.pi * np.cumsum(f) / SR
    return np.tanh(1.5 * np.sin(ph) * np.exp(-t / 0.09))


def clap(seed: int = 0, dur: float = 0.18) -> np.ndarray:
    rng = np.random.default_rng(seed)
    n = int(dur * SR)
    t = np.arange(n) / SR
    noise = rng.normal(0, 1, n)
    bp = noise - onepole_lp(noise, 900)
    bp = onepole_lp(bp, 5000)
    e = np.zeros(n)
    for k, off in enumerate([0, 0.009, 0.018]):
        m = t >= off
        e[m] += (0.7 if k < 2 else 1.0) * np.exp(-(t[m] - off) / (0.006 if k < 2 else 0.05))
    return bp * e


def hat(seed: int = 0, dur: float = 0.06, open_: bool = False) -> np.ndarray:
    rng = np.random.default_rng(seed)
    n = int((0.25 if open_ else dur) * SR)
    t = np.arange(n) / SR
    noise = rng.normal(0, 1, n)
    hp = noise - onepole_lp(noise, 7000)
    return hp * np.exp(-t / (0.08 if open_ else 0.015))


def place(bus: np.ndarray, y: np.ndarray, t: float, gain_db: float = 0.0, pan: float = 0.0) -> None:
    """Suma `y` (mono) en `bus` (n×2) en el segundo t, con ganancia y paneo."""
    p = int(round(t * SR))
    if p < 0:
        y, p = y[-p:], 0
    if p >= len(bus):
        return
    y = y[:len(bus) - p] * 10 ** (gain_db / 20)
    l, r = np.cos((pan + 1) * np.pi / 4), np.sin((pan + 1) * np.pi / 4)
    bus[p:p + len(y), 0] += y * l
    bus[p:p + len(y), 1] += y * r


def fm_bell(midi: float, dur: float = 1.2, ratio: float = 1.5, index: float = 3.0, soft: float = 0.5) -> np.ndarray:
    """Campana FM de 2 operadores (portadora:moduladora 2:3), índice que cae rápido: redonda, no metálica."""
    f = hz(midi)
    n = int(dur * SR)
    t = np.arange(n) / SR
    idx = index * np.exp(-t / (0.05 + 0.15 * soft))
    y = np.sin(2 * np.pi * f * t + idx * np.sin(2 * np.pi * f * ratio * t))
    return y * env_exp(n, dur * 0.3, 0.002 + 0.006 * soft)


def wood(seed: int = 0, f0: float = 520, dur: float = 0.12) -> np.ndarray:
    """Golpe de madera/cerámica: seno que cae + ruido en banda, muy corto."""
    rng = np.random.default_rng(seed)
    n = int(dur * SR)
    t = np.arange(n) / SR
    ph = 2 * np.pi * np.cumsum(f0 * (1 + 0.6 * np.exp(-t / 0.004))) / SR
    tone = np.sin(ph) * np.exp(-t / 0.035)
    noise = rng.normal(0, 1, n)
    noise = onepole_lp(noise - onepole_lp(noise, 1200), 4500) * np.exp(-t / 0.008)
    return tone + 0.5 * noise
