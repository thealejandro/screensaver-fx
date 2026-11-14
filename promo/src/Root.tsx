// Composiciones: Promo-<Reel|Wide|Estado>-<es|en>, Cover-<es|en> (1080 × 1920) y Poster-<es|en> (1920 × 1080).
import React from 'react';
import {Composition, Still} from 'remotion';
import {FORMATOS, FormatoProvider, NOMBRE_FORMATO, TAMANO, type Formato} from './formato';
import {LANGS, LangProvider, type Lang} from './i18n';
import {FPS, TL} from './lib';
import {Portada} from './Portadas';
import {Promo} from './Promo';
import {totalFrames} from './Timeline';

const conContexto = (lang: Lang, formato: Formato, C: React.FC) => {
  const Comp: React.FC = () => (
    <LangProvider lang={lang}>
      <FormatoProvider formato={formato}>
        <C />
      </FormatoProvider>
    </LangProvider>
  );
  return Comp;
};

export const Root: React.FC = () => (
  <>
    {LANGS.flatMap((lang) => [
      ...FORMATOS.map((f) => (
        <Composition
          key={`${f}-${lang}`}
          id={`Promo-${NOMBRE_FORMATO[f]}-${lang}`}
          component={conContexto(lang, f, Promo)}
          durationInFrames={totalFrames(TL, f)}
          fps={FPS}
          {...TAMANO[f]}
        />
      )),
      <Still key={`cover-${lang}`} id={`Cover-${lang}`} component={conContexto(lang, 'reel', Portada)} {...TAMANO.reel} />,
      <Still key={`poster-${lang}`} id={`Poster-${lang}`} component={conContexto(lang, 'wide', Portada)} {...TAMANO.wide} />,
    ])}
  </>
);
