import React from 'react';
import {AbsoluteFill, Audio, Easing, staticFile} from 'remotion';
import {TransitionSeries, linearTiming} from '@remotion/transitions';
import type {TransitionPresentation, TransitionPresentationComponentProps} from '@remotion/transitions';
import {slide} from '@remotion/transitions/slide';
import {useFormato, type Formato} from './formato';

/** Momentos de una escena, en pulsos desde su inicio (un número o un tramo [desde, hasta] o una lista). */
export type Marcas = Record<string, number | number[]>;
export type Entry = {id: string; beats: number; tr?: number; fx?: string; marcas?: Marcas};
/** Línea de tiempo de un producto: fotogramas por pulso de la música y escenas en pulsos, por formato. */
export type TimelineData = {beat: number} & Partial<Record<Formato, Entry[]>>;
/** Escena: recibe su duración en fotogramas (con la transición de salida) y sus marcas. */
export type SceneRender = (dur: number, marcas: Marcas) => React.ReactNode;
export type Music = {src: string; volume: number};

export const entriesOf = (tl: TimelineData, formato: Formato): Entry[] => {
  const list = tl[formato];
  if (!list) throw new Error(`La línea de tiempo no tiene el formato «${formato}»`);
  return list;
};

/** Fotograma (absoluto) en el que empieza cada escena: siempre en un pulso. */
export const sceneStarts = (tl: TimelineData, formato: Formato) => {
  const out: Record<string, number> = {};
  let t = 0;
  for (const e of entriesOf(tl, formato)) {
    out[e.id] = t;
    t += e.beats * tl.beat;
  }
  out._end = t;
  return out;
};

/** Duración total: la suma de los pulsos (las transiciones se solapan dentro de la escena saliente). */
export const totalFrames = (tl: TimelineData, formato: Formato) => entriesOf(tl, formato).reduce((a, e) => a + e.beats * tl.beat, 0);

// ---------------------------------------------------------------------------
// «cover»: la escena nueva aparece encima (opacidad, escala y desenfoque) mientras la anterior
// sigue entera debajo y se aleja un poco. Nunca pasa por negro.
// ---------------------------------------------------------------------------
const Cover: React.FC<TransitionPresentationComponentProps<Record<string, never>>> = ({
  children,
  presentationDirection,
  presentationProgress: p,
}) => {
  if (presentationDirection === 'exiting') {
    return <AbsoluteFill style={{transform: `scale(${1 + 0.05 * p})`, filter: `brightness(${1 - 0.35 * p})`}}>{children}</AbsoluteFill>;
  }
  return (
    <AbsoluteFill
      style={{
        opacity: Math.min(1, p * 1.6),
        transform: `scale(${1.06 - 0.06 * p})`,
        filter: p < 1 ? `blur(${(1 - p) * 16}px)` : undefined,
      }}
    >
      {children}
    </AbsoluteFill>
  );
};
const cover = (): TransitionPresentation<Record<string, never>> => ({component: Cover, props: {}});

const presentation = (fx?: string) => (fx === 'slide' ? slide({direction: 'from-right'}) : cover());
const EASE_T = Easing.bezier(0.33, 0, 0.15, 1);

export const Timeline: React.FC<{
  data: TimelineData;
  scenes: Record<string, SceneRender>;
  music: Music | null;
}> = ({data, scenes, music}) => {
  const list = entriesOf(data, useFormato());
  return (
    <AbsoluteFill style={{background: '#000'}}>
      <TransitionSeries>
        {list.flatMap((e, i) => {
          const tr = i < list.length - 1 ? (e.tr ?? 0) : 0;
          const dur = e.beats * data.beat + tr;
          const render = scenes[e.id];
          if (!render) throw new Error(`Falta la escena «${e.id}»`);
          const seq = (
            <TransitionSeries.Sequence key={e.id} name={e.id} durationInFrames={dur} premountFor={30}>
              {render(dur, e.marcas ?? {})}
            </TransitionSeries.Sequence>
          );
          if (!tr) return [seq];
          return [
            seq,
            <TransitionSeries.Transition
              key={`${e.id}-tr`}
              timing={linearTiming({durationInFrames: tr, easing: EASE_T})}
              presentation={presentation(e.fx) as TransitionPresentation<Record<string, unknown>>}
            />,
          ];
        })}
      </TransitionSeries>
      {/* Música, efectos y nivel ya van mezclados en el audio (scripts/audio/<producto>.py). */}
      {music ? <Audio src={staticFile(music.src)} volume={music.volume} /> : null}
    </AbsoluteFill>
  );
};
