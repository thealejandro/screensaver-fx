// Escenas con las animaciones reales del motor a pantalla completa: gancho, efectos, temas y texto propio.
import React from 'react';
import {AbsoluteFill, Sequence, useCurrentFrame} from 'remotion';
import {useLang, useT} from '../i18n';
import {B, C, Clip, Etiqueta, FadeIn, MONO, Words, num, ramp, useIsVertical} from '../lib';
import {TEMAS, TEXTOS, efecto} from '../textos';
import type {Marcas} from '../Timeline';

/** Titular arriba (dentro de la zona segura) sobre la animación. */
const Arriba: React.FC<{children: React.ReactNode}> = ({children}) => {
  const v = useIsVertical();
  return (
    <AbsoluteFill style={{justifyContent: 'flex-start', alignItems: 'center', paddingTop: v ? 300 : 64}}>{children}</AbsoluteFill>
  );
};

/**
 * Etiqueta abajo: en vertical, por encima del 35 % inferior. Con `bajoTitular` (efectos en vertical) va
 * justo bajo un titular de una línea, para no pisar el reloj, que ocupa la parte baja del rótulo.
 */
const Abajo: React.FC<{children: React.ReactNode; bajoTitular?: boolean}> = ({children, bajoTitular}) => {
  const v = useIsVertical();
  const arriba = v && bajoTitular;
  return (
    <AbsoluteFill
      style={{
        justifyContent: arriba ? 'flex-start' : 'flex-end',
        alignItems: 'center',
        paddingTop: arriba ? 430 : 0,
        paddingBottom: arriba ? 0 : v ? 720 : 96,
      }}
    >
      {/* Placa en esmalte: la etiqueta se lee aunque la animación pase por debajo. */}
      <div style={{background: 'rgba(14, 12, 11, 0.82)', borderRadius: 10, padding: v ? '8px 20px' : '6px 16px'}}>{children}</div>
    </AbsoluteFill>
  );
};

export const Gancho: React.FC<{marcas: Marcas}> = ({marcas}) => {
  const t = useT(TEXTOS);
  const v = useIsVertical();
  const at = num(marcas.titulo) * B;
  return (
    <AbsoluteFill style={{background: '#000'}}>
      <Clip src={v ? 'v/gancho' : 'h/gancho'} />
      <Arriba>
        <FadeIn start={at - 6}>
          <Etiqueta size={v ? 30 : 26} color={C.lente} style={{textAlign: 'center', marginBottom: v ? 26 : 18}}>
            {t.eyebrow}
          </Etiqueta>
        </FadeIn>
        {/* En horizontal, en una línea: el rótulo del motor ocupa el centro de la pantalla. */}
        <Words text={v ? t.gancho : t.gancho.replace('\n', ' ')} start={at} size={v ? 96 : 72} />
      </Arriba>
    </AbsoluteFill>
  );
};

const NOMBRE_CLIP = (id: string, v: boolean, lang: string) => `${v ? 'v' : 'h'}/fx-${id}${id === 'clock' ? `-${lang}` : ''}`;

/** Efectos en cortes secos cada `cada` pulsos, con su nombre y su número. */
export const Efectos: React.FC<{ids: string[]; titular: string; marcas: Marcas; desde?: number; numeros?: number[]}> = ({
  ids,
  titular,
  marcas,
  desde = 0,
  numeros,
}) => {
  const lang = useLang();
  const v = useIsVertical();
  const cada = num(marcas.cada, 2) * B;
  return (
    <AbsoluteFill style={{background: '#000'}}>
      {ids.map((id, i) => (
        <Sequence key={id} from={i * cada} durationInFrames={i === ids.length - 1 ? undefined : cada} premountFor={20}>
          <Clip src={NOMBRE_CLIP(id, v, lang)} />
          <Abajo bajoTitular>
            <Etiqueta size={v ? 34 : 28}>
              <span style={{color: C.lente}}>{String(numeros ? numeros[i] : desde + i + 1).padStart(2, '0')}</span>
              <span style={{color: C.faint}}> / 16 · </span>
              {efecto(id)[lang]}
            </Etiqueta>
          </Abajo>
        </Sequence>
      ))}
      <Arriba>
        {/* En horizontal, a 60 px: con el reloj, el rótulo del motor sube y no debe tocar el titular. */}
        <Words text={titular} start={2} size={v ? 88 : 60} />
      </Arriba>
    </AbsoluteFill>
  );
};

/** El mismo plasma cambiando de tema en cada corte (vertical). */
export const TemasVertical: React.FC<{temas: string[]; marcas: Marcas}> = ({temas, marcas}) => {
  const t = useT(TEXTOS);
  const cada = num(marcas.cada, 2) * B;
  return (
    <AbsoluteFill style={{background: '#000'}}>
      {temas.map((id, i) => (
        <Sequence key={id} from={i * cada} durationInFrames={i === temas.length - 1 ? undefined : cada} premountFor={20}>
          <Clip src={`v/tema-${id}`} />
          <Abajo>
            <Etiqueta size={34}>{TEMAS[id]}</Etiqueta>
          </Abajo>
        </Sequence>
      ))}
      <Arriba>
        <Words text={t.temas} start={2} size={80} />
      </Arriba>
    </AbsoluteFill>
  );
};

const REJILLA = ['aero-night', 'obsidiana', 'ventura', 'big-sur', 'fluent', 'cupertino', 'graphite', 'tahoe', 'aero', 'fluent-light', 'cupertino-light', 'auto'];

/** Los 11 temas y el color de acento en una rejilla de 4 × 3 (horizontal). */
export const TemasRejilla: React.FC<{marcas: Marcas}> = ({marcas}) => {
  const t = useT(TEXTOS);
  const frame = useCurrentFrame();
  const filas = (marcas.filas as number[]) ?? [0, 1, 2];
  const W = 420;
  const H = 236;
  const G = 18;
  return (
    <AbsoluteFill style={{background: C.esmalte}}>
      <AbsoluteFill style={{alignItems: 'center', paddingTop: 70}}>
        <Words text={t.temas.replace('\n', ' ')} start={2} size={60} />
      </AbsoluteFill>
      <div
        style={{
          position: 'absolute',
          left: (1920 - (4 * W + 3 * G)) / 2,
          top: 214,
          display: 'grid',
          gridTemplateColumns: `repeat(4, ${W}px)`,
          gap: G,
        }}
      >
        {REJILLA.map((id, i) => {
          const p = ramp(frame, filas[Math.floor(i / 4)] * B + (i % 4) * 3, filas[Math.floor(i / 4)] * B + (i % 4) * 3 + 14);
          const auto = id === 'auto';
          return (
            <div
              key={id}
              style={{
                width: W,
                height: H,
                position: 'relative',
                borderRadius: 14,
                overflow: 'hidden',
                opacity: p,
                transform: `translateY(${(1 - p) * 18}px)`,
                outline: auto ? `3px solid ${C.lente}` : `1px solid ${C.line}`,
                outlineOffset: -1,
                background: '#000',
              }}
            >
              <Clip src={`t/tema-${id}`} />
              <Etiqueta size={20} color={auto ? C.lente : C.loza} style={{position: 'absolute', left: 14, bottom: 10}}>
                {auto ? t.auto : TEMAS[id]}
              </Etiqueta>
            </div>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};

/** Rótulo propio (CAFÉ / BRB) y luego arte ASCII propio. */
export const Texto: React.FC<{marcas: Marcas}> = ({marcas}) => {
  const t = useT(TEXTOS);
  const lang = useLang();
  const v = useIsVertical();
  const frame = useCurrentFrame();
  const arte = num(marcas.arte) * B;
  const p = ramp(frame, arte - 6, arte + 6);
  const o = v ? 'v' : 'h';
  return (
    <AbsoluteFill style={{background: '#000'}}>
      <Sequence durationInFrames={arte + 6} premountFor={20}>
        <Clip src={`${o}/texto-${lang}`} />
      </Sequence>
      <Sequence from={arte - 6} premountFor={30}>
        <AbsoluteFill style={{opacity: p}}>
          <Clip src={`${o}/arte`} />
        </AbsoluteFill>
      </Sequence>
      <Arriba>
        <div style={{position: 'relative', width: '100%', display: 'flex', justifyContent: 'center'}}>
          <Words text={t.texto} start={num(marcas.rotulo) * B} exitAt={arte - 10} size={v ? 88 : 72} />
          <div style={{position: 'absolute', top: 0, left: 0, right: 0, display: 'flex', justifyContent: 'center'}}>
            <Words text={t.arte} start={arte} size={v ? 88 : 72} />
          </div>
        </div>
      </Arriba>
      {/* Nombre del archivo que se edita: así se lee qué es el «arte ASCII». */}
      <Sequence from={arte + 8}>
        <Abajo>
          <FadeIn start={0}>
            <Etiqueta size={v ? 30 : 26} color={C.dim} style={{fontFamily: MONO}}>
              screensaver.txt
            </Etiqueta>
          </FadeIn>
        </Abajo>
      </Sequence>
    </AbsoluteFill>
  );
};
