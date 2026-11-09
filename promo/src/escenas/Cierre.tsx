// Instalación (terminal recreada), corte en silencio y cierre con el enlace y la firma.
import React from 'react';
import {AbsoluteFill, Img, staticFile, useCurrentFrame} from 'remotion';
import {useT} from '../i18n';
import {B, C, FONT, FadeIn, Fondo, MONO, Recreacion, Words, num, ramp, useIsVertical, EASE_OUT} from '../lib';
import {CMD_MAC, CMD_WIN, REPO, TEXTOS, WEB} from '../textos';
import type {Marcas} from '../Timeline';

const Terminal: React.FC<{
  titulo: string;
  prompt: string;
  cmd: string;
  lineas: string[];
  teclea: number;
  salen: number[];
  w: number;
  size: number;
  alto: number;
}> = ({titulo, prompt, cmd, lineas, teclea, salen, w, size, alto}) => {
  const frame = useCurrentFrame();
  const TYPE = 40;
  const n = Math.round(ramp(frame, teclea, teclea + TYPE, (x) => x) * cmd.length);
  const tecleando = frame < teclea + TYPE + 6;
  const cursor = Math.floor(frame / 16) % 2 === 0;
  return (
    <div
      style={{
        width: w,
        height: alto,
        boxSizing: 'border-box',
        background: C.esmalte,
        border: `2px solid ${C.line}`,
        borderRadius: 14,
        overflow: 'hidden',
        fontFamily: MONO,
        fontSize: size,
        lineHeight: 1.45,
        color: C.loza,
      }}
    >
      <div style={{padding: `${size * 0.5}px ${size}px`, borderBottom: `2px solid ${C.line}`, color: C.dim, fontSize: size * 0.8}}>{titulo}</div>
      <div style={{padding: `${size * 0.8}px ${size}px`, whiteSpace: 'pre-wrap', wordBreak: 'break-all'}}>
        <span style={{color: C.lente}}>{prompt}</span>
        {cmd.slice(0, n)}
        {tecleando && cursor ? <span style={{color: C.lente}}>▌</span> : null}
        {frame >= salen[0] - B / 2 ? (
          <div style={{marginTop: size * 0.8, wordBreak: 'normal'}}>
            <div style={{color: C.lente}}>{'  ❯ Screensaver FX'}</div>
            {lineas.map((l, i) =>
              frame >= salen[i] ? (
                <div key={i} style={{paddingLeft: '2ch', textIndent: 0, color: i === lineas.length - 1 ? C.loza : C.dim}}>
                  {l}
                </div>
              ) : null,
            )}
          </div>
        ) : null}
      </div>
    </div>
  );
};

export const Instalar: React.FC<{marcas: Marcas}> = ({marcas}) => {
  const t = useT(TEXTOS);
  const v = useIsVertical();
  const teclea = num(marcas.teclea) * B;
  const salen = (marcas.lineas as number[]).map((x) => x * B);
  const mac = marcas.mac === undefined ? null : num(marcas.mac) * B;
  return (
    <Fondo>
      <AbsoluteFill style={{alignItems: 'center', paddingTop: v ? 300 : 80}}>
        <Words text={t.instalar} start={2} size={v ? 80 : 68} />
        <FadeIn start={14} style={{marginTop: v ? 18 : 12}}>
          <div style={{fontFamily: FONT, fontSize: v ? 36 : 30, color: C.dim, textAlign: 'center'}}>{t.nativo}</div>
        </FadeIn>
        <FadeIn start={8} style={{marginTop: v ? 30 : 26}}>
          <Recreacion text={t.recreacion} size={v ? 24 : 20} />
        </FadeIn>
        <FadeIn start={4} style={{marginTop: v ? 26 : 24, display: 'flex', gap: 32}}>
          <Terminal
            titulo="PowerShell"
            prompt="PS> "
            cmd={CMD_WIN}
            lineas={t.terminal.win}
            teclea={teclea}
            salen={salen}
            w={v ? 950 : 860}
            size={v ? 28 : 22}
            alto={v ? 560 : 470}
          />
          {mac !== null ? (
            <Terminal
              titulo="zsh"
              prompt="% "
              cmd={CMD_MAC}
              lineas={t.terminal.mac}
              teclea={mac}
              salen={[mac + 1.5 * B, mac + 2.25 * B, mac + 3 * B]}
              w={860}
              size={22}
              alto={470}
            />
          ) : null}
        </FadeIn>
      </AbsoluteFill>
    </Fondo>
  );
};

export const Corte: React.FC = () => {
  const t = useT(TEXTOS);
  const v = useIsVertical();
  return (
    <Fondo>
      <AbsoluteFill style={{justifyContent: 'center', alignItems: 'center', paddingBottom: v ? 220 : 0}}>
        <Words text={t.corte} start={0} stagger={3} size={v ? 120 : 112} />
      </AbsoluteFill>
    </Fondo>
  );
};

export const Cierre: React.FC<{marcas: Marcas}> = ({marcas}) => {
  const t = useT(TEXTOS);
  const v = useIsVertical();
  const frame = useCurrentFrame();
  const firma = num(marcas.firma) * B;
  const p = ramp(frame, 0, 26, EASE_OUT);
  const [usuario, repo] = [REPO.slice(0, REPO.lastIndexOf('/') + 1), REPO.slice(REPO.lastIndexOf('/') + 1)];
  return (
    <Fondo>
      <AbsoluteFill style={{alignItems: 'center', justifyContent: v ? 'flex-start' : 'center', paddingTop: v ? 520 : 0}}>
        <div style={{opacity: p, transform: `translateY(${(1 - p) * 24}px)`, textAlign: 'center', fontFamily: MONO}}>
          <div style={{fontSize: v ? 44 : 44, color: C.dim}}>{usuario}</div>
          <div style={{fontSize: v ? 100 : 120, fontWeight: 700, color: C.loza, letterSpacing: '-0.02em', lineHeight: 1.1}}>{repo}</div>
        </div>
        <FadeIn start={firma} style={{marginTop: v ? 110 : 90, display: 'flex', flexDirection: 'column', alignItems: 'center'}}>
          <Img src={staticFile('marca/logotipo.svg')} style={{width: v ? 500 : 520}} />
          <div style={{marginTop: 22, fontFamily: MONO, fontSize: v ? 34 : 32, color: C.loza}}>{WEB}</div>
        </FadeIn>
        <FadeIn start={firma + 10} style={{marginTop: v ? 90 : 70, textAlign: 'center', fontFamily: FONT, fontSize: v ? 26 : 24, lineHeight: 1.5, color: C.faint}}>
          <div style={{color: C.dim}}>{t.sistemas}</div>
          <div>{t.credito}</div>
          <div>{t.sinRelacion}</div>
        </FadeIn>
      </AbsoluteFill>
    </Fondo>
  );
};
