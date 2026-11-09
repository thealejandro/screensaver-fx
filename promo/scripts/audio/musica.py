#!/usr/bin/env python3
"""Sonido de Screensaver FX (reel, wide y estado): música propia + efectos + máster.

    VCSL=<muestras/vcsl> python3 -I scripts/audio/musica.py <muestras/ir> [--format reel|wide|estado] [--stems DIR] [--out FILE]

Música hecha con el motor de scripts/audio/motor/ (muestras grabadas de VCSL, CC0, y groove humano del
Groove MIDI Dataset), a 150 BPM: un pulso = 24 fotogramas, un compás = 1,6 s. Sin IA, sin pistas de
terceros y sin pads ni tonos sintetizados de fondo. Re mayor (la afinación diatónica del strumstick).
Timbre propio, distinto de los de XACode (piano de cola, vibráfono, arpa con piano vertical):
  - balafón (baqueta blanda) para el motivo, en registro medio (Fa#4–Mi5): notas cortas que se apagan solas;
  - strumstick pulsado con el dedo para los acordes (golpes de 0,1–0,2 s) y figuras sueltas;
  - marimba grave (Outrigger) como bajo, corto;
  - cajón (bajo y borde), palmas en 2 y 4 y un shaker pequeño suelto (dos o tres golpes por compás).
Las tres muestras con altura (balafón, strumstick y marimba) suenan una octava por encima del nombre del
archivo (medido por autocorrelación: «C4» del balafón suena a 521 Hz, «D2» del strumstick a 148 Hz y «C2»
de la marimba a 132 Hz); el sampler pide la nota 12 semitonos más abajo (OCTAVA = -12).

Motivo propio (no el de XACode): Re Re Mi · La Si Re, en 3 + 3 + 2 semicorcheas, como un cursor que
parpadea; la respuesta baja (Do# Si La · Fa# La Fa#). Progresión Re · Si m · Sol · La; el respiro y la
subida van sobre Sol y La, que se cortan en seco antes de volver a Re en el cierre.

Estilos de compás (cambian cada uno o dos compases y dejan huecos):
  gancho   el motivo desde el fotograma 0 y su respuesta; acordes cortos; el shaker anuncia la entrada
  a        acordes a contratiempo, cajón en 1 y 2½, palmas en 2 y 4, shaker suelto
  b        figuras sueltas del strumstick, acordes en 1 y 3, cajón con golpe de borde en el 4¾, palmas
  hueco    solo bajo, un acorde y una figura: un compás para respirar sin percusión
  respiro  acordes suaves y el motivo más bajo; una palma en el 4: el respiro antes del cierre
  sube     corcheas del strumstick en crescendo, cajón y palmas que se doblan hasta el corte seco
  cierre   la cabeza del motivo con palmas en grupo y Re add9 apagado a mano
Entre el corte y el cierre hay silencio (con «Gratis y de código abierto.» y un golpe grave).

Cortes, cierre y cues salen de src/timeline.json (inicio de cada escena y sus `marcas`).
"""
from __future__ import annotations

import math
import os
import sys
from types import SimpleNamespace

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, 'motor'))
from mezcla import load_timeline, run, sec, starts  # noqa: E402
from motor import I, Feel, Hits, VelSampler, place, room, roll  # noqa: E402
from promolib import FPS, SR  # noqa: E402

TL = load_timeline()
B = TL['beat']

BPM = 150
BEAT = 60 / BPM
T = lambda bar, b=0: (bar * 4 + b) * BEAT  # noqa: E731

BALAFON = I + 'Balafon/Soft Mallet'
STRUM = 'Chordophones/Composite Chordophones/Strumstick/Finger'
MARIMBA = I + 'Marimba'
OCTAVA = -12  # nombre del archivo = nota real - 12 en las tres (medido)
SALA = 'Nice Drum Room.wav'

# Acordes: (bajo de marimba, voces del strumstick entre Fa#3 y Fa#4). El motivo queda por encima.
RE = (38, [57, 62, 66])     # Re · La Re Fa#
SIM = (47, [54, 59, 62])    # Si · Fa# Si Re
SOL = (43, [55, 59, 62])    # Sol · Sol Si Re
LA = (45, [57, 61, 64])     # La · La Do# Mi
MIM = (40, [55, 59, 64])    # Mi · Sol Si Mi
FIN = (38, [57, 62, 64, 66])  # Re add9 (final)
CICLO = [RE, SIM, SOL, LA]

# Motivo: (pulso, nota, duración en s). Re Re Mi · La Si Re en 3 + 3 + 2 semicorcheas.
MOTIVO = [(0, 74, .3), (0.75, 74, .24), (1.5, 76, .3), (2, 69, .3), (2.75, 71, .24), (3.25, 74, .5)]
RESPUESTA = [(0, 73, .3), (0.75, 71, .24), (1.5, 69, .3), (2, 66, .3), (2.75, 69, .24), (3.25, 66, .5)]
CABEZA = [(0, 74, .26), (0.75, 74, .24), (1.5, 76, .34)]

# Figuras del strumstick por estilo: (pulso, índice de voz del acorde; +12 = una octava arriba).
FIG_A = [(0.75, 2), (2.25, 1), (3.5, 2)]
FIG_B = [(0, 1), (0.75, 2), (1.5, 14), (3, 2), (3.5, 1)]
FIG_HUECO = [(0, 2), (1, 1), (2.5, 0)]


def suave(x: np.ndarray, fc: float) -> np.ndarray:
    """Paso bajo por FFT con caída suave desde fc (para que el shaker no chille)."""
    n = 1 << (len(x) - 1).bit_length()
    X = np.fft.rfft(x, n)
    fr = np.fft.rfftfreq(n, 1 / SR)
    X *= 1 / (1 + (fr / fc) ** 4)
    return np.fft.irfft(X, n)[:len(x)]


def banda(f: Feel, bus: np.ndarray) -> SimpleNamespace:
    """Instrumentos y gestos comunes (mismo orden de creación: el groove es reproducible)."""
    bal = VelSampler(BALAFON, ['vl1', 'vl2', 'vl3'])
    strum = VelSampler(STRUM, ['vl1', 'vl2'])
    mar = VelSampler(MARIMBA, ['soft', 'med'])
    clap = Hits(I + 'Claps')
    cajon = Hits(I + 'Cajon')
    shaker = Hits(I + 'Shaker, Small')
    cache: dict[str, list] = {}

    def at(bar, b, part='charles', amount=0.6, lag=0.0):
        return 0.0 if (bar, b) == (0, 0) else max(0.0, f.t(bar, b, part, amount, lag))

    def chord(bar, b, ch, vel, dur_s=0.14, gain=-10):
        """Acorde del strumstick rasgueado hacia abajo y apagado enseguida (fundido de 40 ms)."""
        t0 = at(bar, b, lag=0.005)
        voces = ch[1]
        for j, d in enumerate(roll(len(voces), 0.009)):
            place(bus, strum(voces[j] + OCTAVA, vel * (1.05 if j == len(voces) - 1 else 1), dur_s, 0.04), t0 + d, gain,
                  -.25 + .2 * j)

    def bass(bar, b, ch, vel, dur_s=0.26, gain=-5):
        place(bus, mar(ch[0] + OCTAVA, vel, dur_s, 0.07), at(bar, b, 'bombo', 0.5, lag=0.008), gain, -.08)

    def nota(bar, b, midi, vel, dur_s=0.3, gain=-6, pan=.15):
        """Balafón: golpe de baqueta blanda; la lama se apaga con la mano."""
        place(bus, bal(midi + OCTAVA, vel, dur_s, 0.08), at(bar, b, amount=0.6, lag=0.003), gain, pan)

    def motif(bar, frase=MOTIVO, base=0.62, gain=-4):
        for i, (b, n, d) in enumerate(frase):
            v = f.v(bar, b, 'charles', base * [0.95, 0.82, 1.0, 0.92, 0.84, 1.02][i % 6], swing=0.2)
            nota(bar, b, n, v, d, gain, .1 + .06 * (i % 2))

    def figura(bar, ch, pat, base=0.4, gain=-12):
        for b, vi in pat:
            n = ch[1][vi % 12] + (12 if vi >= 12 else 0)
            place(bus, strum(n + OCTAVA, f.v(bar, b, 'charles', base, swing=0.3), 0.2, 0.06), at(bar, b, lag=0.004), gain,
                  .3 - .5 * (b / 4))

    def claps(bar, b, base=0.8, gain=-12):
        t = f.t(bar, b, 'caja', 0.8, lag=-0.004)
        v = f.v(bar, b, 'caja', base, swing=0.4)
        place(bus, clap('Clap_rr', 0.5) * v, t, gain, -.12)
        place(bus, clap('SoloClap_vl[23]', 0.35) * v, t + 0.006, gain - 6, .2)

    def bajo(bar, b, base=0.7, gain=-10):
        """Cajón en el centro de la tapa (golpe grave)."""
        v = f.v(bar, b, 'bombo', base, .3)
        place(bus, cajon('Cajon_hit[23]_(mp|f)', 0.45) * v, at(bar, b, 'bombo', .6), gain, 0)

    def borde(bar, b, base=0.6, gain=-17):
        """Cajón en el borde (golpe corto y más claro)."""
        v = f.v(bar, b, 'caja', base, .3)
        place(bus, cajon('Cajon_hit1_(mp|f)_', 0.25) * v, f.t(bar, b, 'caja', .6), gain, .1)

    def shake(bar, b, base=0.6, gain=-25, pan=.4):
        if 'sh' not in cache:
            cache['sh'] = [suave(shaker('Mid_ShakerLowFaster', 0.16), 4500) for _ in range(4)]
        k = len(cache.setdefault('i', [])) % 4
        cache['i'].append(0)
        x = cache['sh'][k]
        place(bus, x / (np.abs(x).max() + 1e-9) * f.v(bar, b, 'charles', base, .3), f.t(bar, b, 'charles', .6), gain, pan)

    def cierre(bar, end, dur):
        """La cabeza del motivo con palmas en grupo y Re add9 que se apaga a mano antes del final."""
        motif(bar, CABEZA, base=0.7)
        for b, g in ((0, -13), (0.5, -15), (1, -12)):
            claps(bar, b, 0.85, g)
        bajo(bar, 0, 0.8, -9)
        t5 = end + 2 * BEAT
        largo = min(1.5, dur - t5 - 0.3)
        voces = FIN[1]
        for j, d in enumerate(roll(len(voces), 0.03)):
            place(bus, strum(voces[j] + OCTAVA, 0.42 - 0.03 * j, largo, 0.5), t5 + d, -9, -.3 + .2 * j)
        place(bus, mar(FIN[0] + OCTAVA, 0.5, largo, 0.5), t5, -5, -.08)
        place(bus, bal(74 + OCTAVA, 0.5, largo, 0.5), t5 + 0.02, -6, .12)
        place(bus, bal(69 + OCTAVA, 0.4, largo, 0.5), t5 + 0.03, -9, -.1)

    return SimpleNamespace(at=at, chord=chord, bass=bass, nota=nota, motif=motif, figura=figura, claps=claps,
                           bajo=bajo, borde=borde, shake=shake, cierre=cierre)


def plan(formato: str) -> dict:
    """Estilo y acorde por compás, corte seco y motivos de cada formato (compases de 4 pulsos)."""
    sc = starts(TL, formato)
    dur = sec(sc['_end'])
    corte = sec(sc['corte'])
    cierre = sc['cierre'] / (4 * B)
    assert cierre == int(cierre), 'el cierre tiene que empezar en un compás'
    M, R = MOTIVO, RESPUESTA
    if formato == 'reel':
        # gancho 0–1 · efectos 2–5 · temas 6–8 · texto 9–10 · config 11–12 · instalar 13–14 · corte 15 · cierre 16
        estilos = ['gancho'] * 2 + ['a', 'b', 'a', 'b'] + ['a', 'b', 'hueco'] + ['a', 'b'] + ['b', 'a'] + ['respiro', 'sube']
        acordes = [CICLO[i % 4] for i in range(13)] + [SOL, LA]
        motivos = [(0, M, 0.62), (1, R, 0.56), (4, M, 0.56), (9, M, 0.58), (11, R, 0.54), (13, M, 0.42)]
        arch = ((0, 2, 6, 9, 12, 14.9, 16, 18.5), (0.9, 0.86, 0.88, 0.9, 0.82, 1.0, 0.9, 0.8))
        seed = 71
    elif formato == 'wide':
        # gancho 0–2 · clásicos 3–5 · nuevos 6–10 · temas 11–14 · texto 15–17 · config 18–22 · instalar 23–26 ·
        # corte 27 · cierre 28
        estilos = (['gancho'] * 2 + ['a'] + ['b', 'a', 'b'] + ['a', 'b', 'hueco', 'a', 'b'] + ['a', 'b', 'a', 'hueco'] +
                   ['a', 'b', 'b'] + ['a', 'hueco', 'b', 'a', 'b'] + ['a', 'b', 'respiro', 'sube'])
        acordes = [CICLO[i % 4] for i in range(25)] + [SOL, LA]
        motivos = [(0, M, 0.62), (1, R, 0.56), (3, M, 0.56), (6, M, 0.56), (9, R, 0.54), (11, M, 0.56),
                   (15, M, 0.58), (18, M, 0.56), (21, R, 0.54), (25, M, 0.42)]
        arch = ((0, 3, 6, 11, 15, 18, 23, 25, 26.9, 28, 33), (0.9, 0.86, 0.88, 0.9, 0.88, 0.84, 0.86, 0.8, 1.0, 0.9, 0.8))
        seed = 73
    elif formato == 'estado':
        # gancho 0–1 · efectos 2–4 · temas 5–6 · corte 7 · cierre 8
        estilos = ['gancho'] * 2 + ['a', 'b', 'a'] + ['respiro', 'sube']
        acordes = [RE, SIM, SOL, LA, RE, SOL, LA]
        motivos = [(0, M, 0.62), (1, R, 0.56), (3, M, 0.56), (5, M, 0.42)]
        arch = ((0, 2, 5, 6.9, 8, 10.5), (0.9, 0.86, 0.82, 1.0, 0.9, 0.8))
        seed = 79
    else:
        raise ValueError(formato)
    n = math.ceil(corte / T(1) - 1e-9)
    assert len(estilos) == len(acordes) == n, (len(estilos), len(acordes), n)
    # El cierre del horizontal dura 8 s: tras el corte vuelve el groove tres compases (motivo, respuesta y un
    # hueco) antes de la cabeza del motivo y el acorde final, para que la firma no quede en silencio.
    despues = ['a', 'b', 'hueco'] if formato == 'wide' else []
    estilos = estilos + ['corte'] * (int(cierre) - n) + despues
    acordes = acordes + [RE] * (int(cierre) - n) + [RE, SIM, SOL][:len(despues)]
    if despues:
        motivos += [(int(cierre), M, 0.6), (int(cierre) + 1, R, 0.54)]
    return dict(estilos=estilos, acordes=acordes, corte=corte, motivos=motivos, arch=arch, seed=seed,
                cierre=int(cierre), final=int(cierre) + len(despues), dur=dur)


def music(irs: str, formato: str = 'reel') -> np.ndarray:
    p = plan(formato)
    dur, END, est, CUT = p['dur'], T(p['cierre']), p['estilos'], p['corte']
    f = Feel(BPM, seed=p['seed'], off=0.0, arch=p['arch'])
    bus = np.zeros((int(dur * SR) + SR, 2))
    k = banda(f, bus)
    suena = lambda t: t < CUT - 0.05 or t >= END  # noqa: E731
    motivo_en = {bar for bar, _, _ in p['motivos']}

    def sube_de(bar, t):
        a = bar
        while a > 0 and est[a - 1] == 'sube':
            a -= 1
        return 0.72 + 0.28 * (t - T(a)) / (CUT - T(a))

    for bar, (st, ch) in enumerate(zip(est, p['acordes'])):
        if st == 'corte':
            continue
        if bar == p['cierre']:  # vuelta tras el silencio (solo el horizontal): palmas en grupo
            for b, g in ((0, -13), (0.5, -15)):
                k.claps(bar, b, 0.85, g)
        # strumstick (acordes) y marimba (bajo): cada estilo su patrón
        if st == 'gancho':
            golpes, bajos = [(0, 0.2), (1.5, 0.12), (2.5, 0.12), (3.5, 0.1)], [0, 2.5]
        elif st == 'a':
            golpes, bajos = [(0.5, 0.12), (1.5, 0.1), (2.5, 0.12), (3.5, 0.1)], [0, 2.5]
        elif st == 'b':
            golpes, bajos = [(0, 0.16), (2, 0.12)], [0, 1.5, 3]
        elif st == 'sube':
            golpes, bajos = [(i / 2, 0.13 if i % 2 == 0 else 0.09) for i in range(8)], [0, 2]
        elif st == 'hueco':
            golpes, bajos = [(0, 0.28)], [0]
        else:  # respiro
            golpes, bajos = [(0, 0.3), (2.5, 0.2)], [0]
        for b, d in golpes:
            t0 = T(bar, b)
            if not suena(t0):
                continue
            base = {'respiro': 0.34, 'hueco': 0.4}.get(st, 0.5 if b == int(b) else 0.4)
            if st == 'sube':
                base *= sube_de(bar, t0)
            g = {'respiro': -16, 'hueco': -13}.get(st, -10)
            k.chord(bar, b, ch, f.v(bar, b, 'charles', base, swing=0.25), d, g)
        for b in bajos:
            if suena(T(bar, b)):
                k.bass(bar, b, ch, f.v(bar, b, 'bombo', 0.4 if st in ('respiro', 'hueco') else 0.55),
                       0.36 if st == 'respiro' else 0.24, -9 if st == 'respiro' else -5)
        # figuras sueltas del strumstick donde no está el motivo
        if bar not in motivo_en and st in ('a', 'b', 'hueco'):
            pat = {'a': FIG_A, 'b': FIG_B, 'hueco': FIG_HUECO}[st]
            if all(suena(T(bar, b)) for b, _ in pat):
                k.figura(bar, ch, pat, 0.4 if st != 'hueco' else 0.46)

        # percusión
        if st == 'gancho':
            if bar == len([s for s in est if s == 'gancho']) - 1:
                k.shake(bar, 3, 0.6, -24)
                k.shake(bar, 3.5, 0.7, -23)
                k.borde(bar, 3.75, 0.6, -18)
            continue
        if st == 'hueco':
            continue
        if st == 'respiro':
            if suena(T(bar, 3)):
                k.claps(bar, 3, 0.6, -16)
            continue
        ultimo = st == 'sube' and (bar + 1 >= len(est) or est[bar + 1] != 'sube')
        for b in (1, 3):
            if suena(T(bar, b)):
                k.claps(bar, b, 0.78, -12)
        if st == 'a':
            k.bajo(bar, 0, 0.7)
            k.bajo(bar, 2.5, 0.6, -12)
            for b in ((1.5, 3.5) if bar % 2 == 0 else (0.5, 2, 3.5)):
                k.shake(bar, b, 0.55)
        if st == 'b':
            k.bajo(bar, 0, 0.66)
            k.borde(bar, 3.75, 0.6)
            if bar % 2 == 1:
                k.shake(bar, 2.5, 0.5)
        if st == 'sube':
            for b in ((0, 2) if not ultimo else (0, 1.5, 2, 2.5, 3, 3.5)):
                if suena(T(bar, b)):
                    k.bajo(bar, b, 0.55 + 0.25 * (b / 4), -12)
            for b in ((0.5, 2.5) if not ultimo else (2, 2.5, 3.5)):
                if suena(T(bar, b)):
                    k.claps(bar, b, 0.55 + 0.25 * (b / 4), -15)
            if ultimo:
                for b in (3, 3.25, 3.5, 3.75):
                    if suena(T(bar, b)):
                        k.borde(bar, b, 0.5 + 0.12 * (b - 3), -19)
    for bar, frase, base in p['motivos']:
        k.motif(bar, frase, base=base, gain=-10 if est[bar] == 'respiro' else -4)  # el respiro, más bajo

    k.cierre(p['final'], T(p['final']), dur)
    bus = bus[:int(dur * SR)]
    # Sala con envío bajo y sin graves (antes del corte: la cola también se corta en seco).
    bus = room(bus, os.path.join(irs, SALA), -18, predelay=0.008)
    # Silencio real tras el corte (sin fundido largo).
    a, z = int(CUT * SR), int(END * SR) - int(0.004 * SR)
    fade = int(0.01 * SR)
    bus[a:a + fade] *= np.linspace(1, 0, fade)[:, None]
    bus[a + fade:z] = 0
    return bus


PEAK = dict(tick=-28, tick_s=-33, whoosh_s=-34, sub=-16, sub_s=-19)


def cues(formato: str = 'reel') -> list[dict]:
    """Efectos en los momentos de cada escena (inicio de la escena + sus marcas de timeline.json)."""
    out: list[dict] = []
    sc = starts(TL, formato)

    def add(kind, frame, why, **kw):
        out.append(dict(kind=kind, t=frame / FPS, frame=frame, why=why, **kw))

    for e in TL[formato]:
        s, m, n = sc[e['id']], e.get('marcas', {}), e['beats']
        at = lambda x: s + x * B  # noqa: E731
        if e['id'] == 'gancho':
            add('sub', 0, 'fotograma 0: el rótulo se arma con el motivo', f0=58, f1=36)
            add('tick_s', at(m['titulo']), 'entra el titular', pitch=1.0)
        elif e['id'] in ('efectos', 'clasicos', 'nuevos'):
            for i in range(1, int(n / m['cada'])):
                add('tick' if i % 2 else 'tick_s', at(i * m['cada']), f'corte al efecto {i + 1}', pitch=0.94 + 0.03 * (i % 4))
        elif e['id'] == 'temas':
            if 'cada' in m:
                for i in range(1, int(n / m['cada'])):
                    add('tick_s', at(i * m['cada']), f'cambia al tema {i + 1}', pitch=1.02 + 0.02 * (i % 3))
            for i, x in enumerate(m.get('filas', [])):
                add('tick_s', at(x), f'fila {i + 1} de la rejilla de temas', pitch=0.98 + 0.03 * i)
        elif e['id'] == 'texto':
            add('tick', at(m['rotulo']), 'rótulo propio', pitch=0.96)
            add('tick_s', at(m['arte']), '«o tu arte ASCII»', pitch=1.04)
        elif e['id'] == 'config':
            add('whoosh_s', at(m['mac']), 'de Windows a macOS', dur=0.6, lo=400, hi=1800, p0=-0.3, p1=0.3)
        elif e['id'] == 'instalar':
            add('tick', at(m['teclea']), 'pega el comando', pitch=0.96)
            for i, x in enumerate(m['lineas']):
                add('tick_s', at(x), f'línea {i + 1} del instalador', pitch=1.0 + 0.03 * i)
            if 'mac' in m:
                add('tick', at(m['mac']), 'pestaña de macOS', pitch=0.92)
        elif e['id'] == 'corte':
            add('sub_s', s, '«Gratis y de código abierto.»: golpe en el silencio', f0=62, f1=36, decay=0.6)
    return sorted(out, key=lambda x: x['t'])


if __name__ == '__main__':
    run(music, cues, PEAK)
