// La pieza: escenas de cada formato sobre la línea de tiempo (src/timeline.json) y la mezcla de audio.
import React from 'react';
import {useFormato, type Formato} from './formato';
import {useT} from './i18n';
import {TL} from './lib';
import {Timeline, type SceneRender} from './Timeline';
import {Efectos, Gancho, TemasRejilla, TemasVertical, Texto} from './escenas/Animaciones';
import {Config} from './escenas/Config';
import {Cierre, Corte, Instalar} from './escenas/Cierre';
import {EFECTOS, TEXTOS} from './textos';

const REEL_FX = ['neon', 'matrix', 'rain', 'burn', 'glitch', 'waves', 'beams', 'clock'];
const ESTADO_FX = ['matrix', 'burn', 'neon', 'waves'];
const CLASICOS = ['neon', 'starfield', 'terminal', 'glitch', 'plasma', 'clock'];
const NUEVOS = ['decrypt', 'rain', 'beams', 'slide', 'scatter', 'burn', 'matrix', 'waves', 'spotlights', 'wipe'];
const TEMAS_REEL = ['aero-night', 'obsidiana', 'ventura', 'big-sur', 'fluent', 'cupertino'];

/** Número del efecto en la lista completa (core/src/fx/mod.rs): los clásicos van del 1 al 6. */
const ORDEN = EFECTOS.map((e) => e.id);

const useScenes = (formato: Formato): Record<string, SceneRender> => {
  const t = useT(TEXTOS);
  const comunes: Record<string, SceneRender> = {
    gancho: (_d, m) => <Gancho marcas={m} />,
    corte: () => <Corte />,
    cierre: (_d, m) => <Cierre marcas={m} />,
    texto: (_d, m) => <Texto marcas={m} />,
    config: (_d, m) => <Config marcas={m} />,
    instalar: (_d, m) => <Instalar marcas={m} />,
  };
  if (formato === 'wide') {
    return {
      ...comunes,
      clasicos: (_d, m) => <Efectos ids={CLASICOS} titular={t.clasicos} marcas={m} />,
      nuevos: (_d, m) => <Efectos ids={NUEVOS} titular={t.nuevos} marcas={m} desde={6} />,
      temas: (_d, m) => <TemasRejilla marcas={m} />,
    };
  }
  const fx = formato === 'reel' ? REEL_FX : ESTADO_FX;
  return {
    ...comunes,
    efectos: (_d, m) => <EfectosNumerados ids={fx} titular={t.animaciones} marcas={m} />,
    temas: (_d, m) => <TemasVertical temas={formato === 'reel' ? TEMAS_REEL : TEMAS_REEL.slice(0, 4)} marcas={m} />,
  };
};

/** En reel y estado los efectos salen salteados: cada uno lleva su número real. */
const EfectosNumerados: React.FC<{ids: string[]; titular: string; marcas: Record<string, number | number[]>}> = ({ids, titular, marcas}) => (
  <Efectos ids={ids} titular={titular} marcas={marcas} numeros={ids.map((id) => ORDEN.indexOf(id) + 1)} />
);

export const Promo: React.FC = () => {
  const formato = useFormato();
  const scenes = useScenes(formato);
  return <Timeline data={TL} scenes={scenes} music={{src: `audio/mix-${formato}.m4a`, volume: 1}} />;
};
