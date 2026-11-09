// Formato de la pieza. Es una prop (no se deduce del tamaño): reel y estado miden igual.
import {createContext, createElement, useContext} from 'react';
import type {ReactNode} from 'react';

export type Formato = 'reel' | 'wide' | 'estado';
export const FORMATOS: Formato[] = ['reel', 'wide', 'estado'];

/** Nombre del formato en los ids de composición: `<Id>-<Reel|Wide|Estado>-<lang>`. */
export const NOMBRE_FORMATO: Record<Formato, string> = {reel: 'Reel', wide: 'Wide', estado: 'Estado'};

/** Tamaño de la composición de cada formato. */
export const TAMANO: Record<Formato, {width: number; height: number}> = {
  reel: {width: 1080, height: 1920},
  wide: {width: 1920, height: 1080},
  estado: {width: 1080, height: 1920},
};

const FormatoContext = createContext<Formato>('reel');

export const FormatoProvider = ({formato, children}: {formato: Formato; children: ReactNode}) =>
  createElement(FormatoContext.Provider, {value: formato}, children);

export const useFormato = (): Formato => useContext(FormatoContext);
