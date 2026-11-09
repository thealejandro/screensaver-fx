// Recreaciones (no capturas) de la ventana de configuración de Windows y de la hoja de macOS, con las
// cadenas reales del código. Estilizadas en los tres colores de la pieza, sin logotipos ni botones de
// ventana de ningún sistema y marcadas como «Recreación».
import React, {useEffect, useState} from 'react';
import {AbsoluteFill, continueRender, delayRender, staticFile, useCurrentFrame} from 'remotion';
import {useT} from '../i18n';
import {B, C, Clip, FONT, MONO, Recreacion, Words, num, ramp, useIsVertical, EASE_OUT} from '../lib';
import {TEXTOS} from '../textos';
import type {Marcas} from '../Timeline';

/** Contenido de public/arte.txt (el mismo archivo que usa el motor en scripts/clips.py). */
export const useArte = () => {
  const [arte, setArte] = useState('');
  const [handle] = useState(() => delayRender('arte.txt'));
  useEffect(() => {
    fetch(staticFile('arte.txt'))
      .then((r) => r.text())
      .then((s) => {
        setArte(s.replace(/\n+$/, ''));
        continueRender(handle);
      })
      .catch(() => continueRender(handle));
  }, [handle]);
  return arte;
};

const Caja: React.FC<{x: number; y: number; w: number; h?: number; S: number; children?: React.ReactNode; foco?: boolean; combo?: boolean; tenue?: boolean}> = ({
  x,
  y,
  w,
  h = 24,
  S,
  children,
  foco,
  combo,
  tenue,
}) => (
  <div
    style={{
      position: 'absolute',
      left: x * S,
      top: y * S,
      width: w * S,
      height: h * S,
      boxSizing: 'border-box',
      border: `${Math.max(1, S)}px solid ${foco ? C.lente : C.line}`,
      borderRadius: 4 * S,
      display: 'flex',
      alignItems: 'center',
      padding: `0 ${8 * S}px`,
      fontSize: 12 * S,
      color: tenue ? C.faint : C.loza,
      whiteSpace: 'nowrap',
      overflow: 'hidden',
    }}
  >
    <span style={{flex: 1}}>{children}</span>
    {combo ? <span style={{color: C.dim, fontSize: 10 * S}}>▾</span> : null}
  </div>
);

const Boton: React.FC<{x: number; y: number; w: number; S: number; children: React.ReactNode; principal?: boolean; h?: number}> = ({
  x,
  y,
  w,
  S,
  children,
  principal,
  h = 26,
}) => (
  <div
    style={{
      position: 'absolute',
      left: x * S,
      top: y * S,
      width: w * S,
      height: h * S,
      boxSizing: 'border-box',
      borderRadius: 4 * S,
      border: `${Math.max(1, S)}px solid ${principal ? C.lente : C.line}`,
      background: principal ? C.lente : 'transparent',
      color: principal ? C.esmalte : C.loza,
      fontSize: 12 * S,
      fontWeight: principal ? 600 : 400,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
    }}
  >
    {children}
  </div>
);

const Label: React.FC<{x: number; y: number; w: number; S: number; children: React.ReactNode; right?: boolean}> = ({x, y, w, S, children, right}) => (
  <div
    style={{
      position: 'absolute',
      left: x * S,
      top: y * S,
      width: w * S,
      height: 24 * S,
      display: 'flex',
      alignItems: 'center',
      justifyContent: right ? 'flex-end' : 'flex-start',
      fontSize: 12 * S,
      color: C.dim,
      whiteSpace: 'nowrap',
    }}
  >
    {children}
  </div>
);

/** Ventana de configuración de Windows: área cliente de 476 × 372 (windows/src/settings.rs). */
export const VentanaWindows: React.FC<{S: number}> = ({S}) => {
  const t = useT(TEXTOS).win;
  const BAR = 34;
  const ys = [16, 50, 82, 158, 192, 226, 260];
  return (
    <div
      style={{
        width: 476 * S,
        height: (372 + BAR) * S,
        position: 'relative',
        background: C.esmalte,
        border: `${Math.max(1, S)}px solid ${C.line}`,
        borderRadius: 8 * S,
        fontFamily: FONT,
        overflow: 'hidden',
      }}
    >
      <div
        style={{
          height: BAR * S,
          display: 'flex',
          alignItems: 'center',
          padding: `0 ${14 * S}px`,
          fontSize: 12 * S,
          color: C.loza,
          borderBottom: `${Math.max(1, S)}px solid ${C.line}`,
        }}
      >
        {t.titulo}
      </div>
      <div style={{position: 'absolute', top: BAR * S, left: 0, right: 0, bottom: 0}}>
        {t.filas.map(([label, valor], i) => (
          <React.Fragment key={label}>
            <Label x={16} y={ys[i]} w={150} S={S}>
              {label}
            </Label>
            <Caja x={170} y={ys[i]} w={290} S={S} combo={i !== 1 && i !== 2} foco={i === 1}>
              {valor}
              {i === 1 ? <span style={{color: C.lente}}>|</span> : null}
            </Caja>
          </React.Fragment>
        ))}
        <Boton x={170} y={114} w={170} S={S}>
          {t.editar}
        </Boton>
        <Boton x={16} y={310} w={100} S={S}>
          {t.probar}
        </Boton>
        <Boton x={254} y={310} w={96} S={S}>
          {t.cancelar}
        </Boton>
        <Boton x={360} y={310} w={100} S={S} principal>
          {t.guardar}
        </Boton>
      </div>
    </div>
  );
};

/** Hoja de opciones de macOS (macos/ConfigSheet.swift): etiquetas a la derecha y arte ASCII editable. */
export const HojaMac: React.FC<{S: number}> = ({S}) => {
  const t = useT(TEXTOS).mac;
  const arte = useArte();
  const W = 520;
  const ROW = 34;
  const ART = 150;
  let y = 22;
  const filas = t.filas.map(([label, valor], i) => {
    const top = y;
    const art = i === 3;
    y += art ? ART + 10 : ROW;
    return {label, valor, top, art, i};
  });
  const H = y + 60;
  return (
    <div
      style={{
        width: W * S,
        height: H * S,
        position: 'relative',
        background: C.esmalte,
        border: `${Math.max(1, S)}px solid ${C.line}`,
        borderRadius: 12 * S,
        fontFamily: FONT,
        overflow: 'hidden',
      }}
    >
      {filas.map(({label, valor, top, art, i}) => (
        <React.Fragment key={label}>
          <Label x={16} y={top} w={130} S={S} right>
            {label}
          </Label>
          {art ? (
            <div
              style={{
                position: 'absolute',
                left: 158 * S,
                top: top * S,
                width: 340 * S,
                height: ART * S,
                boxSizing: 'border-box',
                border: `${Math.max(1, S)}px solid ${C.lente}`,
                borderRadius: 4 * S,
                padding: `${6 * S}px ${8 * S}px`,
                fontFamily: MONO,
                fontSize: 9.6 * S,
                lineHeight: 1.22,
                color: C.loza,
                whiteSpace: 'pre',
                overflow: 'hidden',
              }}
            >
              {arte}
            </div>
          ) : (
            <Caja x={158} y={top} w={340} S={S} combo={i !== 1 && i !== 2} tenue={i === 1 || i === 2}>
              {valor}
            </Caja>
          )}
        </React.Fragment>
      ))}
      <Boton x={W - 22 - 96 - 10 - 96} y={y + 14} w={96} S={S}>
        {t.cancelar}
      </Boton>
      <Boton x={W - 22 - 96} y={y + 14} w={96} S={S} principal>
        {t.guardar}
      </Boton>
    </div>
  );
};

/** Escena: la ventana de Windows sobre la animación atenuada y, en la marca `mac`, la hoja de macOS. */
export const Config: React.FC<{marcas: Marcas}> = ({marcas}) => {
  const t = useT(TEXTOS);
  const v = useIsVertical();
  const frame = useCurrentFrame();
  const mac = num(marcas.mac) * B;
  const pin = ramp(frame, 0, 22, EASE_OUT);
  const sw = ramp(frame, mac - 8, mac + 10, EASE_OUT);
  const Sw = v ? 1.8 : 1.62;
  const Sm = v ? 1.5 : 1.3;
  const top = v ? 300 : 70;
  return (
    <AbsoluteFill style={{background: '#000'}}>
      <AbsoluteFill style={{filter: 'brightness(0.25)'}}>
        <Clip src={v ? 'v/fondo' : 'h/fondo'} />
      </AbsoluteFill>
      <AbsoluteFill style={{alignItems: 'center', paddingTop: top}}>
        <div style={{position: 'relative', width: '100%', display: 'flex', justifyContent: 'center'}}>
          <Words text={t.configWin} start={2} exitAt={mac - 10} size={v ? 76 : 64} />
          <div style={{position: 'absolute', top: 0, left: 0, right: 0, display: 'flex', justifyContent: 'center'}}>
            <Words text={t.configMac} start={mac} size={v ? 76 : 64} />
          </div>
        </div>
        <div style={{marginTop: v ? 34 : 26, opacity: pin}}>
          <Recreacion text={t.recreacion} size={v ? 24 : 20} />
        </div>
        <div style={{position: 'relative', marginTop: v ? 22 : 20, width: '100%', display: 'flex', justifyContent: 'center'}}>
          <div style={{opacity: pin * (1 - sw), transform: `translateY(${(1 - pin) * 40 - sw * 30}px) scale(${1 - sw * 0.04})`}}>
            <VentanaWindows S={Sw} />
          </div>
          <div style={{position: 'absolute', top: 0, opacity: sw, transform: `translateY(${(1 - sw) * 50}px)`}}>
            <HojaMac S={Sm} />
          </div>
        </div>
      </AbsoluteFill>
    </AbsoluteFill>
  );
};
