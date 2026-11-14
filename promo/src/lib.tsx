import React from 'react';
import {
  AbsoluteFill,
  Easing,
  OffthreadVideo,
  continueRender,
  delayRender,
  interpolate,
  spring,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from 'remotion';
import timelineJson from './timeline.json';
import type {TimelineData} from './Timeline';

// ---------------------------------------------------------------------------
// Marca: paleta AZULEJO, tres colores por pieza (esmalte, loza y lente).
// ---------------------------------------------------------------------------
export const C = {
  esmalte: '#0e0c0b',
  loza: '#f4ecdc',
  lente: '#7f98f5',
  /** Loza con transparencia (texto secundario y líneas): no es un color nuevo. */
  dim: 'rgba(244, 236, 220, 0.62)',
  faint: 'rgba(244, 236, 220, 0.3)',
  line: 'rgba(244, 236, 220, 0.16)',
};
export const FONT = 'InterPromo, Inter, system-ui, sans-serif';
export const MONO = 'JetBrainsPromo, "JetBrains Mono", monospace';

// Fuentes empaquetadas en public/fonts (OFL): render determinista y sin red.
const fontHandle = delayRender('fuentes');
Promise.all([
  new FontFace('InterPromo', `url(${staticFile('fonts/Inter-var-latin.woff2')}) format('woff2')`, {weight: '100 900'}).load(),
  new FontFace('JetBrainsPromo', `url(${staticFile('fonts/JetBrainsMono-latin.woff2')}) format('woff2')`, {weight: '100 800'}).load(),
])
  .then((faces) => {
    faces.forEach((f) => document.fonts.add(f));
    continueRender(fontHandle);
  })
  .catch((e) => {
    console.error(e);
    continueRender(fontHandle);
  });

// ---------------------------------------------------------------------------
// Tiempo
// ---------------------------------------------------------------------------
export const FPS = 60;
export const TL = timelineJson as TimelineData;
/** Un pulso de la música (150 BPM) en fotogramas. */
export const B = TL.beat;
export const EASE = Easing.bezier(0.25, 0.1, 0.25, 1);
export const EASE_IO = Easing.bezier(0.65, 0, 0.35, 1);
export const EASE_OUT = Easing.bezier(0.16, 1, 0.3, 1);

export const ramp = (frame: number, a: number, b: number, ease = EASE_IO) =>
  interpolate(frame, [a, b], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp', easing: ease});

export const useSpr = (delay: number, dur = 40) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  return spring({frame: frame - delay, fps, config: {damping: 200}, durationInFrames: dur});
};

export const useIsVertical = () => {
  const {width, height} = useVideoConfig();
  return height > width;
};

/** Marca de una escena: un número de pulsos (o el primero de una lista). */
export const num = (m: number | number[] | undefined, d = 0) => (m === undefined ? d : Array.isArray(m) ? m[0] : m);

// ---------------------------------------------------------------------------
// Clips del motor real (scripts/clips.py → public/clips)
// ---------------------------------------------------------------------------
export const Clip: React.FC<{src: string; style?: React.CSSProperties}> = ({src, style}) => (
  <OffthreadVideo
    src={staticFile(`clips/${src}.mp4`)}
    muted
    style={{width: '100%', height: '100%', objectFit: 'cover', imageRendering: 'pixelated', ...style}}
  />
);

/** Fondo de las escenas sin animación. */
export const Fondo: React.FC<{children?: React.ReactNode}> = ({children}) => (
  <AbsoluteFill style={{background: C.esmalte}}>{children}</AbsoluteFill>
);

// ---------------------------------------------------------------------------
// Texto
// ---------------------------------------------------------------------------
type WordsProps = {
  text: string;
  start: number;
  stagger?: number;
  size: number;
  weight?: number;
  color?: string;
  exitAt?: number;
  style?: React.CSSProperties;
  align?: 'center' | 'left';
  tracking?: string;
  lineHeight?: number;
};

/** Titular palabra a palabra (desenfoque → nítido). */
export const Words: React.FC<WordsProps> = ({
  text,
  start,
  stagger = 4,
  size,
  weight = 650,
  color = C.loza,
  exitAt,
  style,
  align = 'center',
  tracking = '-0.035em',
  lineHeight = 1.06,
}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const q = exitAt === undefined ? 0 : ramp(frame, exitAt, exitAt + 14, EASE);
  let idx = 0;
  return (
    <div
      style={{
        fontFamily: FONT,
        fontSize: size,
        fontWeight: weight,
        letterSpacing: tracking,
        lineHeight,
        color,
        textAlign: align,
        opacity: 1 - q,
        filter: q > 0 ? `blur(${q * 12}px)` : undefined,
        ...style,
      }}
    >
      {text.split('\n').map((line, li) => (
        <div key={li} style={{whiteSpace: 'nowrap'}}>
          {line.split(' ').map((w, wi) => {
            const i = idx++;
            const p = spring({frame: frame - start - i * stagger, fps, config: {damping: 200}, durationInFrames: 30});
            return (
              <span
                key={wi}
                style={{
                  display: 'inline-block',
                  opacity: p,
                  filter: p < 1 ? `blur(${(1 - p) * 16}px)` : undefined,
                  transform: `translateY(${(1 - p) * size * 0.25}px)`,
                  marginRight: '0.24em',
                }}
              >
                {w}
              </span>
            );
          })}
        </div>
      ))}
    </div>
  );
};

/** Bloque que entra suave. */
export const FadeIn: React.FC<{children: React.ReactNode; start: number; exitAt?: number; style?: React.CSSProperties; rise?: number}> = ({
  children,
  start,
  exitAt,
  style,
  rise = 18,
}) => {
  const frame = useCurrentFrame();
  const p = useSpr(start, 30);
  const q = exitAt === undefined ? 0 : ramp(frame, exitAt, exitAt + 14, EASE);
  return (
    <div
      style={{
        opacity: p * (1 - q),
        filter: p < 1 || q > 0 ? `blur(${(1 - p) * 8 + q * 8}px)` : undefined,
        transform: `translateY(${(1 - p) * rise}px)`,
        ...style,
      }}
    >
      {children}
    </div>
  );
};

/** Etiqueta en monoespaciada (nombre del efecto o del tema). */
export const Etiqueta: React.FC<{children: React.ReactNode; size: number; color?: string; style?: React.CSSProperties}> = ({
  children,
  size,
  color = C.loza,
  style,
}) => (
  <div style={{fontFamily: MONO, fontSize: size, fontWeight: 500, color, letterSpacing: '0.02em', whiteSpace: 'nowrap', ...style}}>
    {children}
  </div>
);

/** Distintivo «Recreación» de los mockups: contorno en lente, sin relleno. */
export const Recreacion: React.FC<{text: string; size: number; style?: React.CSSProperties}> = ({text, size, style}) => (
  <div
    style={{
      fontFamily: MONO,
      fontSize: size,
      fontWeight: 600,
      color: C.lente,
      border: `2px solid ${C.lente}`,
      borderRadius: size * 0.4,
      padding: `${size * 0.25}px ${size * 0.6}px`,
      letterSpacing: '0.08em',
      textTransform: 'uppercase',
      whiteSpace: 'nowrap',
      ...style,
    }}
  >
    {text}
  </div>
);
