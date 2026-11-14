"""Utilidades de audio comunes a los promos: carga y guardado con ffmpeg, filtros, reverb
sintética, arreglo por compases (rubberband), efectos de síntesis propia y máster a LUFS.

Viene de scripts/make-audio.py de omarchy-promo (MIT). Cada producto tiene su script en
scripts/audio/<producto>.py con su plan (secciones de música, automatización) y sus eventos.
"""
from __future__ import annotations

import os
import re
import subprocess

import numpy as np

SR = 48000
FPS = 60


def sec_of_bar(n: float, bar: float, t0: float) -> float:
    return n * bar + t0


# =============================================================================================
# Utilidades de audio
# =============================================================================================
def run(cmd: list[str]) -> str:
    r = subprocess.run(cmd, check=True, capture_output=True, text=True)
    return r.stdout + r.stderr


def load(path: str) -> np.ndarray:
    raw = subprocess.run(
        ['ffmpeg', '-loglevel', 'error', '-i', path, '-ac', '2', '-ar', str(SR), '-f', 'f32le', '-'],
        check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32).reshape(-1, 2).astype(np.float64)


def save(path: str, x: np.ndarray, extra: list[str] | None = None) -> None:
    data = np.ascontiguousarray(x, dtype=np.float32).tobytes()
    cmd = ['ffmpeg', '-loglevel', 'error', '-y', '-f', 'f32le', '-ar', str(SR), '-ac', '2', '-i', '-']
    cmd += extra or ['-c:a', 'pcm_f32le']
    cmd += [path]
    subprocess.run(cmd, input=data, check=True)


def db(x: float) -> float:
    return 10 ** (x / 20)


def soft(x: np.ndarray, drive: float) -> np.ndarray:
    """Saturación suave (tanh) normalizada."""
    return np.tanh(drive * x) / np.tanh(drive)


def ffilt(x: np.ndarray, fn) -> np.ndarray:
    """Filtro de fase cero por FFT para sonidos cortos: fn(f) -> ganancia lineal."""
    n = len(x)
    m = 1 << int(np.ceil(np.log2(n + 1)))
    f = np.fft.rfftfreq(m, 1 / SR)
    g = fn(np.maximum(f, 1e-3))
    X = np.fft.rfft(x, m, axis=0)
    X *= g[:, None] if x.ndim == 2 else g
    return np.fft.irfft(X, m, axis=0)[:n]


def lp_mag(f, fc, order=3):
    return 1 / np.sqrt(1 + (f / fc) ** (2 * order))


def hp_mag(f, fc, order=2):
    return 1 / np.sqrt(1 + (fc / f) ** (2 * order))


def stft_filter(x: np.ndarray, maskfn, n: int = 4096, hop: int = 1024) -> np.ndarray:
    """Filtro variable en el tiempo: maskfn(tiempos[s], f[Hz]) -> matriz (marcos x bins)."""
    win = np.hanning(n + 1)[:-1]
    L = len(x)
    xp = np.pad(x, ((n, n + hop), (0, 0)))
    F = 1 + (len(xp) - n) // hop
    f = np.fft.rfftfreq(n, 1 / SR)
    f[0] = 1e-3
    y = np.zeros_like(xp)
    norm = np.zeros(len(xp))
    w2 = win ** 2
    B = 256
    for b0 in range(0, F, B):
        idx = np.arange(b0, min(F, b0 + B))
        tc = (idx * hop + n / 2 - n) / SR
        M = maskfn(tc, f)
        for ch in range(x.shape[1]):
            fr = np.stack([xp[i * hop:i * hop + n, ch] for i in idx]) * win
            Y = np.fft.rfft(fr, axis=1) * M
            yr = np.fft.irfft(Y, n, axis=1) * win
            for k, i in enumerate(idx):
                y[i * hop:i * hop + n, ch] += yr[k]
        for i in idx:
            norm[i * hop:i * hop + n] += w2
    y /= np.maximum(norm, 1e-9)[:, None]
    return y[n:n + L]


def curve(points: list[tuple[float, float]], t: np.ndarray, log: bool = False) -> np.ndarray:
    """Automatización por puntos (tiempo, valor); interpolación lineal (o logarítmica)."""
    xp = np.array([p[0] for p in points])
    fp = np.array([p[1] for p in points], dtype=float)
    # pasos: tiempos repetidos -> se separan un pelo para que np.interp los respete
    for i in range(1, len(xp)):
        if xp[i] <= xp[i - 1]:
            xp[i] = xp[i - 1] + 1e-4
    if log:
        return np.exp(np.interp(t, xp, np.log(fp)))
    return np.interp(t, xp, fp)


def make_ir(rt60: float, length: float, seed: int, damp: float = 5000, pre: float = 0.012) -> np.ndarray:
    """Respuesta al impulso sintética (sala pequeña): reflexiones tempranas + cola difusa que
    se oscurece con el tiempo. Estéreo decorrelado, energía unitaria."""
    rng = np.random.default_rng(seed)
    n = int(length * SR)
    t = np.arange(n) / SR
    env = np.exp(-6.91 * t / rt60)
    bright = rng.standard_normal((n, 2))
    dark = ffilt(rng.standard_normal((n, 2)), lambda f: lp_mag(f, damp, 2))
    mixw = np.exp(-t / (rt60 * 0.18))[:, None]
    ir = (bright * mixw + dark * (1 - mixw)) * env[:, None]
    ir *= np.minimum(1, t / 0.004)[:, None]  # ataque suave de la cola
    for d, g in [(0.007, 0.5), (0.013, 0.35), (0.021, 0.3), (0.034, 0.2)]:  # tempranas
        k = int(d * SR)
        ir[k, 0] += g * rng.choice([-1, 1])
        ir[k + int(0.0013 * SR), 1] += g * rng.choice([-1, 1])
    ir = np.pad(ir, ((int(pre * SR), 0), (0, 0)))
    return ir / np.sqrt((ir ** 2).sum() / 2)


def convolve(x: np.ndarray, ir: np.ndarray) -> np.ndarray:
    n = len(x) + len(ir)
    m = 1 << int(np.ceil(np.log2(n)))
    y = np.fft.irfft(np.fft.rfft(x, m, axis=0) * np.fft.rfft(ir, m, axis=0), m, axis=0)
    return y[:len(x)]


def pan(x: np.ndarray, p: float) -> np.ndarray:
    """Mono -> estéreo con ley de potencia constante; p en [-1, 1]."""
    a = (p + 1) * np.pi / 4
    return np.stack([x * np.cos(a), x * np.sin(a)], axis=1) * np.sqrt(2)


def ebur128(path: str) -> dict:
    out = run(['ffmpeg', '-nostats', '-i', path, '-af', 'ebur128=peak=true', '-f', 'null', '-'])
    summ = out[out.rfind('Summary:'):]
    g = lambda k: float(re.search(k + r':\s+(-?[\d.]+|-inf)', summ).group(1).replace('-inf', '-200'))
    return {'I': g('I'), 'LRA': g('LRA'), 'TP': g('Peak')}


# =============================================================================================
# 1. Arreglo de la música
# =============================================================================================
def stretch(src: np.ndarray, a: float, b: float, tempo: float, tmp: str, pad: float = 0.4):
    """Estira src[a..b] (segundos del original) con rubberband R3. Devuelve (audio, índice de a)."""
    i0, i1 = int((a - pad) * SR), int((b + pad) * SR)
    chunk = np.zeros((i1 - i0, 2))
    lo, hi = max(0, i0), min(len(src), i1)
    chunk[lo - i0:hi - i0] = src[lo:hi]
    if abs(tempo - 1) < 1e-6:  # ya está en el tempo del vídeo: sin estirar
        return chunk, int(round(pad * SR))
    fi, fo = os.path.join(tmp, 'rb-in.wav'), os.path.join(tmp, 'rb-out.wav')
    save(fi, chunk)
    run(['rubberband', '-3', '-q', '-T', f'{tempo:.6f}', fi, fo])
    return load(fo), int(round(pad / tempo * SR))


def arrange(src: np.ndarray, sections: list[dict], dur: float, tmp: str, bar: float, t0: float, tempo0: float = 1.0) -> np.ndarray:
    """Coloca secciones de compases en la línea de tiempo.

    Cada sección: a, b (compases del original), t (inicio en el vídeo, s), tempo (por defecto
    tempo0) o 'to' (fin en el vídeo, s: entonces el tempo se calcula para que encaje), fin (fundido de entrada, s),
    fout (fundido de salida al final, s; 0,02 = corte seco), tail (s que sigue sonando tras b
    para fundir con la sección siguiente).
    """
    out = np.zeros((int(round(dur * SR)), 2))
    for sec in sections:
        sa, sb = sec_of_bar(sec['a'], bar, t0), sec_of_bar(sec['b'], bar, t0)
        tempo = sec.get('tempo') or ((sb - sa) / (sec['to'] - sec['t']) if 'to' in sec else tempo0)
        y, k0 = stretch(src, sa, sb, tempo, tmp)
        body = int(round((sb - sa) / tempo * SR))
        tail = int(round(sec.get('tail', 0.0) * SR))
        seg = y[k0:k0 + body + tail].copy()
        n = len(seg)
        fin = max(1, int(sec.get('fin', 0.004) * SR))
        e = np.ones(n)
        k = np.arange(min(fin, n))
        shape = sec.get('fin_shape', 'lin')
        e[:len(k)] = (k / fin) ** 2.2 if shape == 'slow' else np.sin(np.pi / 2 * k / fin) ** 2
        if tail:  # se funde con la siguiente justo después del límite
            kk = np.arange(tail)
            e[body:body + tail] *= np.cos(np.pi / 2 * kk / tail) ** 2
        fout = int(sec.get('fout', 0) * SR)
        if fout:
            kk = np.arange(fout)
            e[body - fout:body] *= np.cos(np.pi / 2 * kk / fout) ** 2
            e[body:] = 0
        if 'decay' in sec:  # caída exponencial (acorde final)
            tt = np.arange(n) / SR
            e *= np.exp(-tt / sec['decay'])
        seg *= e[:, None]
        p = int(round(sec['t'] * SR))
        m = min(n, len(out) - p)
        out[p:p + m] += seg[:m] * db(sec.get('gain', 0))
    return out


# =============================================================================================
# 2. Efectos (síntesis propia)
# =============================================================================================
IR_SMALL = None
IR_HALL = None


def irs():
    global IR_SMALL, IR_HALL
    if IR_SMALL is None:
        IR_SMALL = make_ir(0.35, 0.6, seed=11, damp=6000, pre=0.004)
        IR_HALL = make_ir(1.9, 3.0, seed=13, damp=3500, pre=0.02)
    return IR_SMALL, IR_HALL


def env_ad(n: int, att: float, dec: float) -> np.ndarray:
    t = np.arange(n) / SR
    a = np.minimum(1, t / att)
    return np.sin(np.pi / 2 * a) ** 2 * np.exp(-np.maximum(0, t - att) / dec)


def finish(y: np.ndarray, peak_db: float) -> np.ndarray:
    return y / (np.abs(y).max() + 1e-12) * db(peak_db)


def sfx_tick(rng, pitch=1.0, bright=1.0, p=0.0):
    """Clic de interfaz: transitorio de ruido filtrado + dos modos resonantes amortiguados
    (madera/plástico) + un 'toc' grave, saturación ligera y sala corta."""
    n = int(0.09 * SR)
    t = np.arange(n) / SR
    noise = rng.standard_normal(n) * np.exp(-t / 0.0016)
    noise = ffilt(noise, lambda f: hp_mag(f, 2200, 2) * lp_mag(f, 9000 * bright, 2))
    f1 = 1850 * pitch * (1 + rng.uniform(-0.01, 0.01))
    body = (np.sin(2 * np.pi * f1 * t + rng.uniform(0, 6)) * np.exp(-t / 0.012) * 0.55
            + np.sin(2 * np.pi * f1 * 2.71 * t) * np.exp(-t / 0.005) * 0.25
            + np.sin(2 * np.pi * 410 * pitch * t) * np.exp(-t / 0.009) * 0.35)
    y = noise * 1.2 + body
    y *= np.minimum(1, t / 0.0004)  # sin escalón digital
    y = soft(y / np.abs(y).max(), 1.6)
    st = pan(y, p)
    small, _ = irs()
    st = st + convolve(np.pad(st, ((0, int(0.4 * SR)), (0, 0))), small)[:len(st)] * 0.18
    return st, int(0.0006 * SR)  # el transitorio está al principio


def sfx_whoosh(rng, dur=0.8, peak=0.55, lo=300, hi=2600, p0=-0.5, p1=0.5):
    """Soplido: ruido rosa estéreo con paso banda que sube y baja, envolvente asimétrica y
    movimiento de panorama."""
    n = int(dur * SR)
    t = np.arange(n) / SR
    w = rng.standard_normal((n, 2))
    w = ffilt(w, lambda f: 1 / np.sqrt(np.maximum(f, 20)))  # rosa
    pk = peak * dur

    def mask(tc, f):
        u = np.clip(tc / pk, 0, None)
        fc = np.where(tc < pk, lo * (hi / lo) ** np.clip(u, 0, 1),
                      hi * (lo * 1.6 / hi) ** np.clip((tc - pk) / (dur - pk), 0, 1))[:, None]
        q = np.log2(f[None, :] / fc)
        return np.exp(-0.5 * (q / 0.9) ** 2)

    y = stft_filter(np.pad(w, ((2048, 2048), (0, 0))), mask, n=1024, hop=256)[2048:2048 + n]
    e = np.where(t < pk, (t / pk) ** 2.4, np.exp(-(t - pk) / (0.22 * (dur - pk) + 1e-3)))
    e *= np.minimum(1, (dur - t) / 0.02)  # sin corte al final
    pp = np.interp(t, [0, dur], [p0, p1])
    a = (pp + 1) * np.pi / 4
    y = y * e[:, None]
    y = np.stack([y[:, 0] * np.cos(a) * 1.2 + y[:, 1] * 0.2, y[:, 1] * np.sin(a) * 1.2 + y[:, 0] * 0.2], axis=1)
    return y, int(pk * SR)


def sfx_sub(rng, f0=68, f1=38, decay=0.42, click=0.25):
    """Golpe grave: barrido de tono descendente con envolvente real, transitorio de ruido
    grave y saturación asimétrica (armónicos pares para que se note en altavoces pequeños)."""
    n = int(1.6 * SR)
    t = np.arange(n) / SR
    freq = f1 + (f0 - f1) * np.exp(-t / 0.09)
    ph = 2 * np.pi * np.cumsum(freq) / SR
    y = np.sin(ph) * env_ad(n, 0.004, decay)
    y += 0.12 * np.sin(2 * ph + 0.7) * env_ad(n, 0.004, decay * 0.5)
    nz = rng.standard_normal(n) * np.exp(-t / 0.006)
    y += click * ffilt(nz, lambda f: lp_mag(f, 900, 2) * hp_mag(f, 60, 1))
    y = soft(y + 0.18 * y ** 2, 1.4)
    y = ffilt(y, lambda f: hp_mag(f, 26, 2))
    y *= np.minimum(1, (t[-1] - t) / 0.05)
    st = pan(y, 0)
    small, _ = irs()
    st += convolve(np.pad(st, ((0, int(0.6 * SR)), (0, 0))), small)[:n] * 0.10
    return st, int(0.004 * SR)


def bell(rng, freq, n, gain=1.0, decay=1.1):
    t = np.arange(n) / SR
    out = np.zeros((n, 2))
    # parciales tipo celesta/vibráfono suave: 1, 2, 3, 4,07, 5,4 con caídas más cortas arriba
    for r, a, d in [(1, 1.0, 1.0), (2.0, 0.28, 0.45), (3.0, 0.07, 0.3), (4.07, 0.06, 0.18), (5.4, 0.025, 0.1)]:
        for ch, cents in ((0, -3), (1, 3)):
            fr = freq * r * 2 ** (cents / 1200)
            out[:, ch] += a * np.sin(2 * np.pi * fr * t + rng.uniform(0, 6)) * env_ad(n, 0.007, decay * d)
    mallet = ffilt(rng.standard_normal(n) * np.exp(-t / 0.003), lambda f: lp_mag(f, 3500, 2) * hp_mag(f, 300, 1))
    out += 0.08 * mallet[:, None]
    return soft(out * gain / np.abs(out).max(), 1.3)


def sfx_tone(rng, notes, decay=1.1, wet=0.32):
    """Tono suave (isla): una o dos notas de campana suave con sala larga."""
    n = int(3.2 * SR)
    y = np.zeros((n, 2))
    for freq, off, g in notes:
        k = int(off * SR)
        y[k:] += bell(rng, freq, n - k, g, decay)
    _, hall = irs()
    y = y + convolve(y, hall) * wet
    y *= np.minimum(1, (n / SR - np.arange(n) / SR) / 0.3)[:, None]
    return y, int(0.007 * SR)


def sfx_swell(rng, dur=1.0):
    """Soplido inverso que desemboca en el golpe: ruido con paso alto que sube y corte seco."""
    n = int(dur * SR)
    t = np.arange(n) / SR
    w = ffilt(rng.standard_normal((n, 2)), lambda f: 1 / np.sqrt(np.maximum(f, 20)))

    def mask(tc, f):
        fc = (500 * (5000 / 500) ** np.clip(tc / dur, 0, 1))[:, None]
        return hp_mag(f[None, :], fc, 1) * lp_mag(f[None, :], fc * 3, 1)

    y = stft_filter(np.pad(w, ((2048, 2048), (0, 0))), mask, n=1024, hop=256)[2048:2048 + n]
    e = (t / dur) ** 3.2 * np.minimum(1, (dur - t) / 0.008)
    return y * e[:, None], n  # se alinea por el final


# =============================================================================================
# 3. Mezcla y máster
# =============================================================================================
def master(mix: np.ndarray, out_m4a: str, tmp: str, target=-16.0, ceil_db=-1.6) -> dict:
    """Ganancia + limitador de pico real; se mide sobre el AAC final (el códec mueve unas décimas)."""
    wav = os.path.join(tmp, 'mix.wav')
    lim = os.path.join(tmp, 'lim.wav')
    # 30 ms de entrada (< 2 fotogramas): un golpe a nivel pleno en la primera muestra hace que el AAC
    # al recodificar (WhatsApp, web) rebase 0 dBFS (+1 a +2 dB medidos); con 30 ms queda bajo el techo.
    n = int(0.030 * SR)
    mix = mix.copy()
    mix[:n] *= np.linspace(0, 1, n)[:, None] if mix.ndim == 2 else np.linspace(0, 1, n)
    save(wav, mix)
    gain = target - ebur128(wav)['I']
    for _ in range(8):
        run(['ffmpeg', '-loglevel', 'error', '-y', '-i', wav, '-af',
             f'volume={gain:.3f}dB,aresample={SR * 4},alimiter=limit={db(ceil_db):.4f}:attack=2:release=80:level=0:latency=1,'
             f'aresample={SR}', '-c:a', 'pcm_f32le', lim])
        run(['ffmpeg', '-loglevel', 'error', '-y', '-i', lim, '-c:a', 'aac', '-b:a', '256k', '-movflags', '+faststart', out_m4a])
        m = ebur128(out_m4a)
        if m['TP'] > -1.1:  # el AAC rebasa el techo: se baja y se repite
            ceil_db -= 0.4
            continue
        if abs(m['I'] - target) < 0.1:
            break
        gain += target - m['I']
    return m


