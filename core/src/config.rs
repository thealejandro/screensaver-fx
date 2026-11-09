//! Configuración compartida por todas las plataformas: un ini de `clave=valor`.
//!
//! ```ini
//! theme=auto        # auto (acento del sistema) | random | aero | obsidiana | ...
//! size=m            # s | m | l  (tamaño del rótulo)
//! lang=auto         # auto | es | en  (idioma de la fecha del reloj)
//! effects=all       # all | classic | modern | lista separada por comas: neon,plasma
//! banner=default    # default (JACC) | text (title + subtitle) | file (screensaver.txt)
//! title=JACC
//! subtitle=XACODE
//! ```

use crate::color::{theme_index, Rgb};
use crate::fx::{fx_index, EFFECTS};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeSel {
    Auto,
    Random,
    Named(usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Size {
    S,
    M,
    L,
}

impl Size {
    /// Filas de texto que caben en pantalla a la escala ideal (más filas = rótulo más pequeño).
    pub fn rows(self) -> f32 {
        match self {
            Size::S => 44.0,
            Size::M => 36.0,
            Size::L => 28.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Es,
    En,
}

/// Qué rótulo se anima.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Banner {
    /// JACC + XACODE dibujado a mano.
    Default,
    /// `title` + `subtitle` escritos en la configuración.
    Text,
    /// Arte ASCII propio (screensaver.txt).
    File,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub banner: Banner,
    pub title: String,
    pub subtitle: String,
    pub theme: ThemeSel,
    pub size: Size,
    pub lang: Lang,
    /// Índices en `fx::EFFECTS` de los efectos activos (nunca vacío).
    pub effects: Vec<usize>,
    /// Color de acento del sistema (para `theme=auto`).
    pub accent: Rgb,
    pub seed: u64,
    /// Rótulo propio (contenido de screensaver.txt), si hay.
    pub art: Option<String>,
}

impl Config {
    pub fn parse(ini: &str, accent: Rgb, seed: u64, sys_lang_es: bool) -> Config {
        let mut cfg = Config {
            banner: Banner::Default,
            title: String::new(),
            subtitle: String::new(),
            theme: ThemeSel::Auto,
            size: Size::M,
            lang: if sys_lang_es { Lang::Es } else { Lang::En },
            effects: (0..EFFECTS.len()).collect(),
            accent,
            seed,
            art: None,
        };
        for line in ini.lines() {
            let line = line.trim().trim_start_matches('\u{feff}');
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') || line.starts_with('[') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, raw) = (k.trim().to_ascii_lowercase(), v.trim());
            let v = raw.to_ascii_lowercase();
            match k.as_str() {
                "banner" => {
                    cfg.banner = match v.as_str() {
                        "text" => Banner::Text,
                        "file" => Banner::File,
                        _ => Banner::Default,
                    }
                }
                "title" => cfg.title = raw.to_string(),
                "subtitle" => cfg.subtitle = raw.to_string(),
                "theme" => {
                    cfg.theme = match v.as_str() {
                        "auto" | "system" => ThemeSel::Auto,
                        "random" => ThemeSel::Random,
                        id => theme_index(id).map(ThemeSel::Named).unwrap_or(ThemeSel::Auto),
                    }
                }
                "size" => {
                    cfg.size = match v.as_str() {
                        "s" | "small" => Size::S,
                        "l" | "large" => Size::L,
                        _ => Size::M,
                    }
                }
                "lang" => match v.as_str() {
                    "es" => cfg.lang = Lang::Es,
                    "en" => cfg.lang = Lang::En,
                    _ => {}
                },
                "effects" => {
                    let set: Vec<usize> = match v.as_str() {
                        "all" | "" => (0..EFFECTS.len()).collect(),
                        "classic" => (0..EFFECTS.len()).filter(|&i| EFFECTS[i].classic).collect(),
                        "modern" => (0..EFFECTS.len()).filter(|&i| !EFFECTS[i].classic).collect(),
                        list => list.split(',').filter_map(|s| fx_index(s.trim())).collect(),
                    };
                    if !set.is_empty() {
                        cfg.effects = set;
                    }
                }
                _ => {}
            }
        }
        cfg
    }
}
