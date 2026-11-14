// Portada (1080 × 1920) y póster (1920 × 1080): un fotograma real del motor con el titular y el enlace.
import React from 'react';
import {AbsoluteFill, Img, OffthreadVideo, staticFile} from 'remotion';
import {useT} from './i18n';
import {C, FONT, MONO, useIsVertical} from './lib';
import {REPO, TEXTOS} from './textos';

const Enlace: React.FC<{v: boolean}> = ({v}) => (
  <div style={{display: 'flex', flexDirection: 'column', alignItems: 'center', marginTop: 0}}>
    <div style={{fontFamily: MONO, fontSize: v ? 34 : 32, color: C.loza}}>{REPO}</div>
    <Img src={staticFile('marca/logotipo.svg')} style={{width: v ? 300 : 280, marginTop: v ? 26 : 22, opacity: 0.9}} />
  </div>
);

const Titular: React.FC<{size: number; oneLine?: boolean}> = ({size, oneLine}) => {
  const t = useT(TEXTOS);
  const texto = oneLine ? t.portada.replace('\n', ' ') : t.portada;
  return (
    <div style={{fontFamily: FONT, fontSize: size, fontWeight: 650, letterSpacing: '-0.035em', lineHeight: 1.06, color: C.loza, textAlign: 'center'}}>
      {texto.split('\n').map((l) => (
        <div key={l} style={{whiteSpace: 'nowrap'}}>
          {l}
        </div>
      ))}
    </div>
  );
};

export const Portada: React.FC = () => {
  const t = useT(TEXTOS);
  const v = useIsVertical();
  return (
    <AbsoluteFill style={{background: '#000'}}>
      {/* Fotograma del gancho ya montado (el rótulo FX resuelto). */}
      <OffthreadVideo
        src={staticFile(`clips/${v ? 'v' : 'h'}/gancho.mp4`)}
        muted
        startFrom={v ? 200 : 290}
        style={{width: '100%', height: '100%', objectFit: 'cover', imageRendering: 'pixelated'}}
      />
      <AbsoluteFill style={{background: 'linear-gradient(180deg, rgba(14,12,11,0.85) 0%, rgba(14,12,11,0) 32%, rgba(14,12,11,0) 62%, rgba(14,12,11,0.9) 100%)'}} />
      <AbsoluteFill style={{alignItems: 'center', paddingTop: v ? 300 : 90}}>
        <div style={{fontFamily: MONO, fontSize: v ? 30 : 26, fontWeight: 500, color: C.lente, marginBottom: v ? 26 : 18}}>{t.eyebrow}</div>
        <Titular size={v ? 96 : 76} oneLine={!v} />
      </AbsoluteFill>
      {/* En vertical, el enlace queda bajo el rótulo y dentro del recorte 4:5 de la cuadrícula. */}
      <AbsoluteFill style={{alignItems: 'center', justifyContent: 'flex-end', paddingBottom: v ? 560 : 80}}>
        <Enlace v={v} />
      </AbsoluteFill>
    </AbsoluteFill>
  );
};
