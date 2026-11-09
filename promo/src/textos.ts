// Textos del vídeo en los dos idiomas (de tú). Los de las ventanas recreadas y los nombres de efectos y
// temas son las cadenas reales del código (windows/src/settings.rs, macos/ConfigSheet.swift,
// core/src/fx/mod.rs, core/src/color.rs).
import type {Dict} from './i18n';

export const REPO = 'github.com/thealejandro/screensaver-fx';
export const WEB = 'thealejandro.dev';

/** Efectos: id del motor y nombre en cada idioma (core/src/fx/mod.rs). Los seis primeros son los clásicos. */
export const EFECTOS: {id: string; es: string; en: string}[] = [
  {id: 'neon', es: 'Neón', en: 'Neon'},
  {id: 'starfield', es: 'Estrellas', en: 'Starfield'},
  {id: 'terminal', es: 'Terminal', en: 'Terminal'},
  {id: 'glitch', es: 'Glitch', en: 'Glitch'},
  {id: 'plasma', es: 'Plasma', en: 'Plasma'},
  {id: 'clock', es: 'Reloj', en: 'Clock'},
  {id: 'decrypt', es: 'Descifrado', en: 'Decrypt'},
  {id: 'rain', es: 'Lluvia', en: 'Rain'},
  {id: 'beams', es: 'Rayos', en: 'Beams'},
  {id: 'slide', es: 'Deslizar', en: 'Slide'},
  {id: 'scatter', es: 'Dispersión', en: 'Scatter'},
  {id: 'burn', es: 'Fuego', en: 'Burn'},
  {id: 'matrix', es: 'Matrix', en: 'Matrix'},
  {id: 'waves', es: 'Olas', en: 'Waves'},
  {id: 'spotlights', es: 'Focos', en: 'Spotlights'},
  {id: 'wipe', es: 'Barrido', en: 'Wipe'},
];
export const efecto = (id: string) => EFECTOS.find((e) => e.id === id)!;

/** Temas (core/src/color.rs). `auto` es el color de acento del sistema. */
export const TEMAS: Record<string, string> = {
  aero: 'Aero',
  'aero-night': 'Aero Night',
  obsidiana: 'Obsidiana',
  fluent: 'Fluent',
  'fluent-light': 'Fluent Light',
  tahoe: 'Tahoe',
  ventura: 'Ventura',
  'big-sur': 'Big Sur',
  graphite: 'Graphite',
  cupertino: 'Cupertino',
  'cupertino-light': 'Cupertino Light',
};

type Textos = {
  eyebrow: string;
  gancho: string;
  animaciones: string;
  clasicos: string;
  nuevos: string;
  temas: string;
  auto: string;
  texto: string;
  arte: string;
  configWin: string;
  configMac: string;
  recreacion: string;
  instalar: string;
  nativo: string;
  corte: string;
  sistemas: string;
  credito: string;
  sinRelacion: string;
  portada: string;
  win: {
    titulo: string;
    filas: [string, string][];
    editar: string;
    probar: string;
    cancelar: string;
    guardar: string;
  };
  mac: {
    filas: [string, string][];
    arte: string;
    cancelar: string;
    guardar: string;
  };
  terminal: {
    win: string[];
    mac: string[];
  };
};

export const TEXTOS: Dict<Textos> = {
  es: {
    eyebrow: 'Screensaver FX · Windows y macOS',
    gancho: 'Tu pantalla,\nen modo terminal.',
    animaciones: '16 animaciones.',
    clasicos: 'Seis clásicos.',
    nuevos: 'Y diez nuevos.',
    temas: '11 temas.\nO el color de tu sistema.',
    auto: 'Color de acento',
    texto: 'Con tu propio texto.',
    arte: 'O tu arte ASCII.',
    configWin: 'Configúralo en Windows…',
    configMac: '…y en macOS.',
    recreacion: 'Recreación',
    instalar: 'Una línea para instalarlo.',
    nativo: 'Nativo en Rust. Sin navegador.',
    corte: 'Gratis y de\ncódigo abierto.',
    sistemas: 'Windows 10/11 · macOS 11+',
    credito: 'Inspirado en Omarchy y terminaltexteffects.',
    sinRelacion: 'Sin relación con Microsoft ni Apple.',
    portada: 'Tu pantalla,\nen modo terminal.',
    win: {
      titulo: 'Screensaver FX · Configuración',
      filas: [
        ['Rótulo', 'Texto personalizado'],
        ['Título', 'CAFÉ'],
        ['Subtítulo', 'ya vuelvo'],
        ['Colores', 'Aero Night'],
        ['Tamaño', 'Mediano'],
        ['Efectos', 'Todos'],
        ['Idioma del reloj', 'Del sistema'],
      ],
      editar: 'Editar screensaver.txt…',
      probar: 'Probar',
      cancelar: 'Cancelar',
      guardar: 'Guardar',
    },
    mac: {
      filas: [
        ['Rótulo', 'Arte ASCII propio'],
        ['Título', 'Título (hasta 16 caracteres)'],
        ['Subtítulo', 'Subtítulo (opcional)'],
        ['Arte ASCII', ''],
        ['Colores', 'Cupertino'],
        ['Tamaño', 'Mediano'],
        ['Efectos', 'Clásicos de Omarchy'],
        ['Idioma del reloj', 'Del sistema'],
      ],
      arte: 'Arte ASCII',
      cancelar: 'Cancelar',
      guardar: 'Guardar',
    },
    terminal: {
      win: ['Descargando la última versión…', 'Instalando…', 'Instalado. Configúralo desde Inicio › «Configurar Screensaver FX».', 'Ya es tu protector de pantalla.'],
      mac: ['Descargando la última versión…', 'Instalando en ~/Library/Screen Savers…', 'Instalado. Elígelo en Ajustes del Sistema › Salvapantallas.'],
    },
  },
  en: {
    eyebrow: 'Screensaver FX · Windows and macOS',
    gancho: 'Your screen,\nin terminal mode.',
    animaciones: '16 animations.',
    clasicos: 'Six classics.',
    nuevos: 'And ten new ones.',
    temas: '11 themes.\nOr your system color.',
    auto: 'Accent color',
    texto: 'Your own text.',
    arte: 'Or your own ASCII art.',
    configWin: 'Set it up on Windows…',
    configMac: '…and on macOS.',
    recreacion: 'Recreation',
    instalar: 'One line to install.',
    nativo: 'Native, written in Rust. No browser.',
    corte: 'Free and\nopen source.',
    sistemas: 'Windows 10/11 · macOS 11+',
    credito: 'Inspired by Omarchy and terminaltexteffects.',
    sinRelacion: 'Not affiliated with Microsoft or Apple.',
    portada: 'Your screen,\nin terminal mode.',
    win: {
      titulo: 'Screensaver FX · Settings',
      filas: [
        ['Banner', 'Custom text'],
        ['Title', 'BRB'],
        ['Subtitle', 'back soon'],
        ['Colors', 'Aero Night'],
        ['Size', 'Medium'],
        ['Effects', 'All'],
        ['Clock language', 'System'],
      ],
      editar: 'Edit screensaver.txt…',
      probar: 'Preview',
      cancelar: 'Cancel',
      guardar: 'Save',
    },
    mac: {
      filas: [
        ['Banner', 'Own ASCII art'],
        ['Title', 'Title (up to 16 characters)'],
        ['Subtitle', 'Subtitle (optional)'],
        ['ASCII art', ''],
        ['Colors', 'Cupertino'],
        ['Size', 'Medium'],
        ['Effects', 'Omarchy classics'],
        ['Clock language', 'System'],
      ],
      arte: 'ASCII art',
      cancelar: 'Cancel',
      guardar: 'Save',
    },
    terminal: {
      win: ['Downloading the latest version…', 'Installing…', 'Installed. Set it up from Start › “Configure Screensaver FX”.', 'It’s now your screen saver.'],
      mac: ['Downloading the latest version…', 'Installing to ~/Library/Screen Savers…', 'Installed. Pick it in System Settings › Screen Saver.'],
    },
  },
};

export const CMD_WIN = 'irm https://raw.githubusercontent.com/thealejandro/screensaver-fx/main/install.ps1 | iex';
export const CMD_MAC = 'curl -fsSL https://raw.githubusercontent.com/thealejandro/screensaver-fx/main/install.sh | bash';
