// Idioma de cada composición: Root pone el `Lang` por contexto y las escenas llaman a useT(TEXTOS).
// Mismo patrón que thealejandro/omarchy-promo (src/i18n.tsx, MIT).
import React, {createContext, useContext} from 'react';

export type Lang = 'es' | 'en';
export const LANGS: Lang[] = ['es', 'en'];

const LangContext = createContext<Lang>('es');

export const LangProvider: React.FC<{lang: Lang; children: React.ReactNode}> = ({lang, children}) => (
  <LangContext.Provider value={lang}>{children}</LangContext.Provider>
);

export const useLang = (): Lang => useContext(LangContext);

/** Diccionario con los dos idiomas obligatorios: así ninguna pieza en inglés cae al español. */
export type Dict<T> = Record<Lang, T>;

export const useT = <T,>(dict: Dict<T>): T => dict[useLang()];
