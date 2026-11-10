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

/// Cómo se reparten las animaciones con varios monitores (`screens=` en config.ini).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screens {
    /// `span`: un solo escritorio, sincronizado; los efectos de escritorio pasan de una pantalla
    /// a la siguiente según su posición (Configuración › Pantalla en Windows).
    Span,
    /// `each`: cada pantalla por su cuenta, con su propio orden de efectos.
    Each,
    /// `main`: solo la pantalla principal; las demás quedan en negro.
    Main,
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
    pub screens: Screens,
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
            screens: Screens::Span,
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
                "screens" => {
                    cfg.screens = match v.as_str() {
                        "each" => Screens::Each,
                        "main" => Screens::Main,
                        _ => Screens::Span,
                    }
                }
                _ => {}
            }
        }
        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::THEMES;

    const ACCENT: Rgb = Rgb(0x00, 0x78, 0xD4);

    fn p(ini: &str) -> Config {
        Config::parse(ini, ACCENT, 42, true)
    }

    fn ids(cfg: &Config) -> Vec<&'static str> {
        cfg.effects.iter().map(|&i| EFFECTS[i].id_str()).collect()
    }

    fn all() -> Vec<usize> {
        (0..EFFECTS.len()).collect()
    }

    #[test]
    fn empty_ini_gives_defaults() {
        let c = p("");
        assert_eq!(c.banner, Banner::Default);
        assert_eq!(c.title, "");
        assert_eq!(c.subtitle, "");
        assert_eq!(c.theme, ThemeSel::Auto);
        assert_eq!(c.size, Size::M);
        assert_eq!(c.lang, Lang::Es);
        assert_eq!(c.effects, all());
        assert_eq!(c.accent, ACCENT);
        assert_eq!(c.seed, 42);
        assert!(c.art.is_none());
    }

    #[test]
    fn system_language_is_the_default_lang() {
        assert_eq!(Config::parse("", ACCENT, 0, true).lang, Lang::Es);
        assert_eq!(Config::parse("", ACCENT, 0, false).lang, Lang::En);
        // lang=auto o un valor desconocido no tocan el idioma del sistema.
        assert_eq!(Config::parse("lang=auto", ACCENT, 0, false).lang, Lang::En);
        assert_eq!(Config::parse("lang=fr", ACCENT, 0, true).lang, Lang::Es);
    }

    #[test]
    fn lang_values() {
        assert_eq!(Config::parse("lang=es", ACCENT, 0, false).lang, Lang::Es);
        assert_eq!(Config::parse("lang=en", ACCENT, 0, true).lang, Lang::En);
        assert_eq!(Config::parse("lang=ES", ACCENT, 0, false).lang, Lang::Es);
        assert_eq!(Config::parse("LANG = En ", ACCENT, 0, true).lang, Lang::En);
    }

    #[test]
    fn whitespace_only_or_comment_only_ini_gives_defaults() {
        for ini in ["\n\n   \n\t\n", "# theme=aero\n; size=l\n[saver]\n", "sin igual\n=\n==\n"] {
            let c = p(ini);
            assert_eq!(c.theme, ThemeSel::Auto, "{ini:?}");
            assert_eq!(c.size, Size::M, "{ini:?}");
            assert_eq!(c.banner, Banner::Default, "{ini:?}");
            assert_eq!(c.effects, all(), "{ini:?}");
        }
    }

    #[test]
    fn banner_values() {
        assert_eq!(p("banner=text").banner, Banner::Text);
        assert_eq!(p("banner=file").banner, Banner::File);
        assert_eq!(p("banner=default").banner, Banner::Default);
        assert_eq!(p("banner=TEXT").banner, Banner::Text);
        assert_eq!(p("banner=xyz").banner, Banner::Default);
        assert_eq!(p("banner=").banner, Banner::Default);
        // Un valor inválido posterior vuelve al rótulo por defecto.
        assert_eq!(p("banner=text\nbanner=???").banner, Banner::Default);
    }

    #[test]
    fn title_and_subtitle_keep_case_and_inner_text() {
        let c = p("title=  Hola Mundo  \nsubtitle=\tXaCode 2026 ");
        assert_eq!(c.title, "Hola Mundo");
        assert_eq!(c.subtitle, "XaCode 2026");
        // Solo se parte en el primer '='; '#' y ';' dentro del valor no son comentarios.
        let c = p("title=a=b # c ; d");
        assert_eq!(c.title, "a=b # c ; d");
        // Unicode.
        let c = p("title=Ñandú\nsubtitle=日本語");
        assert_eq!(c.title, "Ñandú");
        assert_eq!(c.subtitle, "日本語");
        // Vaciar explícitamente.
        let c = p("title=X\ntitle=");
        assert_eq!(c.title, "");
    }

    #[test]
    fn title_length_is_not_limited_by_the_parser() {
        // El recorte (MAX_TITLE/MAX_SUBTITLE) lo hace el rótulo, no la configuración.
        let long = "W".repeat(500);
        let c = p(&format!("title={long}\nsubtitle={long}"));
        assert_eq!(c.title.len(), 500);
        assert_eq!(c.subtitle.len(), 500);
    }

    #[test]
    fn theme_values() {
        assert_eq!(p("theme=auto").theme, ThemeSel::Auto);
        assert_eq!(p("theme=system").theme, ThemeSel::Auto);
        assert_eq!(p("theme=random").theme, ThemeSel::Random);
        assert_eq!(p("theme=RANDOM").theme, ThemeSel::Random);
        assert_eq!(p("theme=no-existe").theme, ThemeSel::Auto);
        assert_eq!(p("theme=").theme, ThemeSel::Auto);
        assert_eq!(p("theme=aero").theme, ThemeSel::Named(0));
        assert_eq!(p("theme=Aero-Night").theme, ThemeSel::Named(1));
        // Valor inválido después de uno válido: vuelve a auto.
        assert_eq!(p("theme=aero\ntheme=zzz").theme, ThemeSel::Auto);
    }

    #[test]
    fn every_theme_id_parses_to_its_index() {
        for (i, t) in THEMES.iter().enumerate() {
            assert_eq!(p(&format!("theme={}", t.id_str())).theme, ThemeSel::Named(i));
            assert_eq!(p(&format!("theme={}", t.id_str().to_uppercase())).theme, ThemeSel::Named(i));
            assert_eq!(p(&format!("  Theme =  {}  ", t.id_str())).theme, ThemeSel::Named(i));
        }
    }

    #[test]
    fn size_values() {
        assert_eq!(p("size=s").size, Size::S);
        assert_eq!(p("size=small").size, Size::S);
        assert_eq!(p("size=S").size, Size::S);
        assert_eq!(p("size=l").size, Size::L);
        assert_eq!(p("size=large").size, Size::L);
        assert_eq!(p("size=LARGE").size, Size::L);
        assert_eq!(p("size=m").size, Size::M);
        assert_eq!(p("size=xl").size, Size::M);
        assert_eq!(p("size=").size, Size::M);
        assert_eq!(p("size=l\nsize=42").size, Size::M);
    }

    #[test]
    fn size_rows_are_ordered() {
        // Más filas = rótulo más pequeño: S < M < L en tamaño visual.
        assert!(Size::S.rows() > Size::M.rows());
        assert!(Size::M.rows() > Size::L.rows());
        assert!(Size::L.rows() > 0.0);
    }

    #[test]
    fn effects_all_classic_modern() {
        assert_eq!(p("effects=all").effects, all());
        assert_eq!(p("effects=ALL").effects, all());
        assert_eq!(p("effects=").effects, all());
        let classic = p("effects=classic").effects;
        let modern = p("effects=modern").effects;
        assert_eq!(classic.len(), 6, "los seis efectos de Omarchy");
        assert!(classic.iter().all(|&i| EFFECTS[i].classic));
        assert!(modern.iter().all(|&i| !EFFECTS[i].classic));
        assert_eq!(classic.len() + modern.len(), EFFECTS.len());
        assert!(!modern.is_empty());
    }

    #[test]
    fn effects_list() {
        assert_eq!(ids(&p("effects=neon,plasma")), ["neon", "plasma"]);
        assert_eq!(ids(&p("effects= Neon , PLASMA ,matrix ")), ["neon", "plasma", "matrix"]);
        // Desconocidos y vacíos se ignoran.
        assert_eq!(ids(&p("effects=neon,,nope, ,wipe")), ["neon", "wipe"]);
        // Lista sin ningún efecto válido: se queda la selección anterior (nunca vacío).
        assert_eq!(p("effects=nope,zzz").effects, all());
        assert_eq!(ids(&p("effects=wipe\neffects=nope")), ["wipe"]);
        assert_eq!(p("effects=,,,").effects, all());
    }

    #[test]
    fn every_effect_id_parses() {
        for (i, f) in EFFECTS.iter().enumerate() {
            assert_eq!(p(&format!("effects={}", f.id_str())).effects, vec![i]);
            assert_eq!(p(&format!("EFFECTS={}", f.id_str().to_uppercase())).effects, vec![i]);
        }
    }

    #[test]
    fn effects_never_empty_and_in_range() {
        let inputs = ["effects=", "effects=,", "effects=classic,modern", "effects=all,neon", "effects=\u{feff}", "effects=neon;plasma", "effects=💥"];
        for ini in inputs {
            let c = p(ini);
            assert!(!c.effects.is_empty(), "{ini:?}");
            assert!(c.effects.iter().all(|&i| i < EFFECTS.len()), "{ini:?}");
        }
    }

    #[test]
    fn bom_crlf_case_and_spaces() {
        let ini = "\u{feff}theme=obsidiana\r\n  SIZE  =  L  \r\nBanner=Text\r\nTitle = Hola\r\nlang=en\r\n";
        let c = Config::parse(ini, ACCENT, 1, true);
        assert_eq!(c.theme, ThemeSel::Named(2));
        assert_eq!(c.size, Size::L);
        assert_eq!(c.banner, Banner::Text);
        assert_eq!(c.title, "Hola");
        assert_eq!(c.lang, Lang::En);
    }

    #[test]
    fn screens_option() {
        let s = |ini: &str| Config::parse(ini, ACCENT, 1, true).screens;
        assert_eq!(s(""), Screens::Span);
        assert_eq!(s("screens=MAIN"), Screens::Main);
        assert_eq!(s("screens = each "), Screens::Each);
        assert_eq!(s("screens=span"), Screens::Span);
        assert_eq!(s("screens=quién sabe"), Screens::Span);
    }

    #[test]
    fn bom_before_comment_or_section() {
        let c = p("\u{feff}# comentario\nsize=s");
        assert_eq!(c.size, Size::S);
        let c = p("\u{feff}[Screensaver FX]\nsize=l");
        assert_eq!(c.size, Size::L);
        let c = p("\u{feff}size=s");
        assert_eq!(c.size, Size::S);
    }

    #[test]
    fn comments_and_sections_are_skipped() {
        let c = p("# theme=aero\n; theme=fluent\n[theme=tahoe]\n  # size=l\nsize=s");
        assert_eq!(c.theme, ThemeSel::Auto);
        assert_eq!(c.size, Size::S);
    }

    #[test]
    fn unknown_keys_are_ignored_and_last_value_wins() {
        let c = p("foo=bar\ncolor=red\nsize=l\nsize=s\ntheme=aero\ntheme=fluent");
        assert_eq!(c.size, Size::S);
        assert_eq!(c.theme, ThemeSel::Named(3));
    }

    #[test]
    fn windows_settings_format_round_trip() {
        // El formato que escribe windows/src/settings.rs.
        let ini = "# Screensaver FX\nbanner=text\ntitle=JACC\nsubtitle=XACODE\ntheme=random\nsize=l\neffects=modern\nlang=en\n";
        let c = Config::parse(ini, ACCENT, 9, true);
        assert_eq!(c.banner, Banner::Text);
        assert_eq!((c.title.as_str(), c.subtitle.as_str()), ("JACC", "XACODE"));
        assert_eq!(c.theme, ThemeSel::Random);
        assert_eq!(c.size, Size::L);
        assert!(c.effects.iter().all(|&i| !EFFECTS[i].classic));
        assert_eq!(c.lang, Lang::En);
    }

    #[test]
    fn arbitrary_garbage_never_panics() {
        let mut s = String::new();
        for i in 0..2000u32 {
            s.push(char::from_u32((i * 7919) % 0x3000).unwrap_or('?'));
            if i % 37 == 0 {
                s.push('=');
            }
            if i % 53 == 0 {
                s.push('\n');
            }
        }
        let c = p(&s);
        assert!(!c.effects.is_empty());
        assert!(c.effects.iter().all(|&i| i < EFFECTS.len()));
    }
}
