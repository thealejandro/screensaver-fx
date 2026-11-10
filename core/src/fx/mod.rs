//! Efectos: cada uno dibuja la rejilla completa en cada fotograma a partir de su tiempo local.

mod classic;
mod extra;
mod modern;
mod multi;

use crate::art::Art;
use crate::desk::Desk;
use crate::color::{gradient, mix, Rgb, WHITE};
use crate::grid::Grid;
use crate::rng::{hash3, Rng};
use crate::{Lang, WallTime};
use core::ffi::CStr;

pub struct Ctx {
    pub g: Grid,
    pub art: Art,
    pub pal: Vec<Rgb>,
    pub rng: Rng,
    pub now: WallTime,
    pub lang: Lang,
    /// Esquina del rótulo en la rejilla.
    pub x0: i32,
    pub y0: i32,
    /// Todas las pantallas: los efectos de escritorio dibujan la parte que cae en esta.
    pub desk: Desk,
}

impl Ctx {
    pub fn center(&mut self) {
        self.x0 = ((self.g.cols - self.art.w) / 2).max(0);
        self.y0 = ((self.g.rows - self.art.h) / 2).max(0);
    }
}

pub trait Effect {
    /// `t`: segundos desde que empezó el efecto; `dt`: desde el fotograma anterior.
    fn draw(&mut self, t: f32, dt: f32, c: &mut Ctx);
}

pub struct FxDef {
    pub id: &'static CStr,
    pub name_es: &'static CStr,
    pub name_en: &'static CStr,
    pub secs: f32,
    /// Uno de los seis efectos del screensaver de Omarchy original.
    pub classic: bool,
    pub make: fn(&mut Ctx) -> Box<dyn Effect>,
}

impl FxDef {
    pub fn id_str(&self) -> &'static str {
        self.id.to_str().unwrap_or("")
    }
}

macro_rules! fx {
    ($id:literal, $es:literal, $en:literal, $secs:expr, $classic:expr, $make:path) => {
        FxDef { id: $id, name_es: $es, name_en: $en, secs: $secs, classic: $classic, make: $make }
    };
}

pub static EFFECTS: [FxDef; 25] = [
    fx!(c"neon", c"Neón", c"Neon", 14.0, true, classic::neon),
    fx!(c"starfield", c"Estrellas", c"Starfield", 16.0, true, classic::starfield),
    fx!(c"terminal", c"Terminal", c"Terminal", 10.0, true, classic::terminal),
    fx!(c"glitch", c"Glitch", c"Glitch", 12.0, true, classic::glitch),
    fx!(c"plasma", c"Plasma", c"Plasma", 15.0, true, classic::plasma),
    fx!(c"clock", c"Reloj", c"Clock", 20.0, true, classic::clock),
    fx!(c"decrypt", c"Descifrado", c"Decrypt", 11.0, false, modern::decrypt),
    fx!(c"rain", c"Lluvia", c"Rain", 11.0, false, modern::rain),
    fx!(c"beams", c"Rayos", c"Beams", 11.0, false, modern::beams),
    fx!(c"slide", c"Deslizar", c"Slide", 10.0, false, modern::slide),
    fx!(c"scatter", c"Dispersión", c"Scatter", 11.0, false, modern::scatter),
    fx!(c"burn", c"Fuego", c"Burn", 12.0, false, modern::burn),
    fx!(c"matrix", c"Matrix", c"Matrix", 14.0, false, modern::matrix),
    fx!(c"waves", c"Olas", c"Waves", 12.0, false, modern::waves),
    fx!(c"spotlights", c"Focos", c"Spotlights", 12.0, false, modern::spotlights),
    fx!(c"wipe", c"Barrido", c"Wipe", 10.0, false, modern::wipe),
    fx!(c"relay", c"Relevo", c"Relay", 13.0, false, multi::relay),
    fx!(c"comet", c"Cometa", c"Comet", 12.0, false, multi::comet),
    fx!(c"tide", c"Marea", c"Tide", 12.0, false, multi::tide),
    fx!(c"fireworks", c"Fuegos artificiales", c"Fireworks", 13.0, false, extra::fireworks),
    fx!(c"vortex", c"Remolino", c"Vortex", 11.0, false, extra::vortex),
    fx!(c"bounce", c"Rebote", c"Bounce", 10.0, false, extra::bounce),
    fx!(c"laser", c"Láser", c"Laser", 10.5, false, extra::laser),
    fx!(c"crystal", c"Cristal", c"Crystal", 11.0, false, extra::crystal),
    fx!(c"ripple", c"Onda", c"Ripple", 11.0, false, extra::ripple),
];

pub fn fx_index(id: &str) -> Option<usize> {
    EFFECTS.iter().position(|f| f.id_str().eq_ignore_ascii_case(id))
}

// ── Utilidades ──────────────────────────────────────────────────────────────

pub fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

pub fn ease_out_cubic(p: f32) -> f32 {
    1.0 - (1.0 - clamp01(p)).powi(3)
}

pub fn ease_in_out_cubic(p: f32) -> f32 {
    let p = clamp01(p);
    if p < 0.5 {
        4.0 * p * p * p
    } else {
        1.0 - (-2.0 * p + 2.0).powi(3) / 2.0
    }
}

/// Color final del rótulo: gradiente diagonal suave que se desplaza despacio.
pub fn settle(pal: &[Rgb], art: &Art, x: i32, y: i32, t: f32) -> Rgb {
    gradient(pal, (x as f32 / art.w.max(1) as f32 + y as f32 / art.h.max(1) as f32 * 0.3) * 0.5 + t * 0.02)
}

/// Destello blanco que se funde en `base` durante `dur` segundos.
pub fn flash(base: Rgb, since: f32, dur: f32) -> Rgb {
    mix(WHITE, base, clamp01(since / dur))
}

/// Banda de brillo diagonal que cruza el rótulo entre `t0` y `t1`.
pub fn shine(c: Rgb, art: &Art, x: i32, y: i32, t: f32, t0: f32, t1: f32) -> Rgb {
    if t < t0 || t > t1 {
        return c;
    }
    let span = (art.w + art.h * 2 + 16) as f32;
    let p = (t - t0) / (t1 - t0) * span - 8.0;
    let d = ((x + y * 2) as f32 - p).abs();
    if d < 4.0 {
        mix(c, WHITE, (1.0 - d / 4.0) * 0.7)
    } else {
        c
    }
}

pub const NOISE: &[u8] = b"!#$%&*+-/<=>?@^~0123456789ABCDEFXYZabcdefxyz{}[]|;:";

pub fn noise_ch(a: u32, b: u32, c: u32) -> char {
    NOISE[hash3(a, b, c) as usize % NOISE.len()] as char
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{accent_palette, THEMES};
    use crate::grid::EMPTY;

    fn ctx(cols: i32, rows: i32, art: Art, seed: u64) -> Ctx {
        let (art_w, art_h) = (art.w, art.h);
        let mut c = Ctx {
            g: Grid::new(cols, rows),
            art,
            pal: THEMES[2].palette(),
            rng: Rng::new(seed),
            now: WallTime { year: 2026, month: 11, day: 9, weekday: 0, hour: 21, minute: 47, second: 10 },
            lang: Lang::Es,
            x0: 0,
            y0: 0,
            desk: Desk::single(crate::Layout { cols, rows, buf_w: cols.max(0) * 8, buf_h: rows.max(0) * 16, dst_x: 0, dst_y: 0, dst_w: cols.max(1) * 8, dst_h: rows.max(1) * 16, scaled_down: false }, art_w, art_h),
        };
        c.center();
        c
    }

    /// Ejecuta un efecto completo como lo hace Engine (rejilla limpia en cada fotograma).
    fn run(def: &FxDef, c: &mut Ctx, dt: f32, until: f32) -> Box<dyn Effect> {
        let mut fx = (def.make)(c);
        let mut t = 0.0f32;
        while t < until {
            c.g.clear();
            fx.draw(t, dt, c);
            t += dt;
        }
        fx
    }

    #[test]
    fn effect_table_is_consistent() {
        let mut ids = Vec::new();
        for f in EFFECTS.iter() {
            let id = f.id_str();
            assert!(!id.is_empty() && id == id.to_ascii_lowercase() && !id.contains([',', ' ', '=']), "{id:?}");
            assert!(!ids.contains(&id), "id repetido {id}");
            ids.push(id);
            assert!(!f.name_es.to_bytes().is_empty() && !f.name_en.to_bytes().is_empty());
            assert!(f.name_es.to_str().is_ok() && f.name_en.to_str().is_ok(), "{id}: nombre no UTF-8");
            assert!(f.secs >= 5.0 && f.secs <= 60.0, "{id}: {}", f.secs);
            assert_eq!(fx_index(id), ids.iter().position(|&i| i == id));
        }
        assert_eq!(EFFECTS.iter().filter(|f| f.classic).count(), 6);
        assert_eq!(fx_index("NEON"), Some(0));
        assert_eq!(fx_index(""), None);
        assert_eq!(fx_index("all"), None);
    }

    #[test]
    fn easing_helpers() {
        assert_eq!(clamp01(-1.0), 0.0);
        assert_eq!(clamp01(2.0), 1.0);
        assert_eq!(clamp01(0.25), 0.25);
        for f in [ease_out_cubic, ease_in_out_cubic] {
            assert_eq!(f(0.0), 0.0);
            assert_eq!(f(1.0), 1.0);
            assert_eq!(f(-3.0), 0.0);
            assert_eq!(f(9.0), 1.0);
            let mut last = 0.0;
            for i in 0..=100 {
                let v = f(i as f32 / 100.0);
                assert!(v >= last - 1e-6 && (0.0..=1.0).contains(&v), "monótona en [0,1]");
                last = v;
            }
        }
        assert!((ease_in_out_cubic(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn flash_and_shine() {
        let base = Rgb(10, 100, 200);
        assert_eq!(flash(base, 0.0, 0.5), WHITE);
        assert_eq!(flash(base, 0.5, 0.5), base);
        assert_eq!(flash(base, 10.0, 0.5), base);
        let art = Art::default_art();
        // Fuera de la ventana [t0, t1] no toca el color.
        assert_eq!(shine(base, &art, 3, 3, 1.0, 2.0, 4.0), base);
        assert_eq!(shine(base, &art, 3, 3, 5.0, 2.0, 4.0), base);
        // Dentro, alguna celda se aclara.
        let lit = (0..art.w).any(|x| shine(base, &art, x, 0, 3.0, 2.0, 4.0) != base);
        assert!(lit);
    }

    #[test]
    fn settle_is_safe_with_degenerate_art() {
        let art = Art { w: 0, h: 0, cells: vec![] };
        // w = h = 0 no debe dividir por cero ni devolver negro.
        let c = settle(&THEMES[0].palette(), &art, 5, 5, 1.0);
        assert_ne!(c, Rgb(0, 0, 0));
        assert_eq!(settle(&[], &art, 0, 0, 0.0), WHITE);
    }

    #[test]
    fn noise_chars_are_printable_ascii() {
        for i in 0..2000u32 {
            let ch = noise_ch(i, i.wrapping_mul(31), 7);
            assert!(ch.is_ascii_graphic(), "{ch:?}");
            assert!(NOISE.contains(&(ch as u8)));
        }
    }

    #[test]
    fn every_effect_ends_with_the_full_banner() {
        // Antes de terminar, cada celda del rótulo está en su sitio con su carácter
        // (glitch puede estar en plena ráfaga: basta con la mayoría).
        for seed in [1u64, 2, 3] {
            for def in EFFECTS.iter() {
                let mut c = ctx(120, 40, Art::default_art(), seed);
                run(def, &mut c, 1.0 / 60.0, def.secs - 0.3);
                let ok = c.art.cells.iter().filter(|&&(x, y, ch)| {
                    let cell = c.g.cells[((c.y0 + y) * c.g.cols + c.x0 + x) as usize];
                    cell.ch == ch && cell.fg != 0
                });
                let (ok, n) = (ok.count(), c.art.cells.len());
                if def.id_str() == "glitch" {
                    assert!(ok * 2 > n, "{} seed {seed}: {ok}/{n}", def.id_str());
                } else {
                    assert_eq!(ok, n, "{} seed {seed}: rótulo incompleto", def.id_str());
                }
            }
        }
    }

    #[test]
    fn every_effect_draws_something_early() {
        for def in EFFECTS.iter() {
            let mut c = ctx(120, 40, Art::default_art(), 9);
            run(def, &mut c, 1.0 / 60.0, 2.0);
            assert!(c.g.cells.iter().any(|&cell| cell != EMPTY), "{} no dibuja nada a los 2 s", def.id_str());
        }
    }

    #[test]
    fn effects_survive_tiny_grids_and_odd_arts() {
        let arts = || {
            vec![
                Art::default_art(),
                Art::parse("X").unwrap(),
                Art::parse("█\n\n\n\n█").unwrap(),
                Art::parse(&"▀▄".repeat(60)).unwrap(),
                Art::from_text("Hola", "mundo").unwrap(),
            ]
        };
        for (cols, rows) in [(0, 0), (1, 1), (2, 1), (5, 3), (10, 10), (40, 2), (2, 40), (200, 60)] {
            for (ai, art) in arts().into_iter().enumerate() {
                for def in EFFECTS.iter() {
                    let mut c = ctx(cols, rows, art_clone(&art), ai as u64);
                    // Pasos grandes y pequeños para cubrir saltos de tiempo.
                    run(def, &mut c, 0.37, def.secs + 1.0);
                    run(def, &mut c, 1.0 / 30.0, 3.0);
                    assert_eq!(c.g.cells.len(), (cols.max(0) * rows.max(0)) as usize);
                }
            }
        }
    }

    fn art_clone(a: &Art) -> Art {
        Art { w: a.w, h: a.h, cells: a.cells.clone() }
    }

    #[test]
    fn effects_tolerate_odd_times() {
        for def in EFFECTS.iter() {
            let mut c = ctx(80, 30, Art::default_art(), 4);
            let mut fx = (def.make)(&mut c);
            for (t, dt) in [(0.0, 0.0), (-5.0, 0.0), (1e-9, 1e-9), (3.0, 0.1), (2.0, 0.0), (def.secs, 0.1), (def.secs * 3.0, 0.1), (1e6, 0.1)] {
                c.g.clear();
                fx.draw(t, dt, &mut c);
            }
        }
    }

    #[test]
    fn same_seed_same_frames() {
        for def in EFFECTS.iter() {
            let frames = |seed| {
                let mut c = ctx(90, 30, Art::default_art(), seed);
                let mut fx = (def.make)(&mut c);
                let mut out = Vec::new();
                for i in 0..120 {
                    c.g.clear();
                    fx.draw(i as f32 / 20.0, 1.0 / 20.0, &mut c);
                    out.push(c.g.cells.clone());
                }
                out
            };
            assert!(frames(11) == frames(11), "{} no es determinista", def.id_str());
        }
    }

    #[test]
    fn random_effects_depend_on_the_seed() {
        for id in ["neon", "starfield", "decrypt", "rain", "scatter", "matrix", "burn"] {
            let def = &EFFECTS[fx_index(id).unwrap()];
            let frame = |seed| {
                let mut c = ctx(90, 30, Art::default_art(), seed);
                run(def, &mut c, 1.0 / 30.0, 1.5);
                c.g.cells.clone()
            };
            assert!(frame(1) != frame(2), "{id}: la semilla no cambia nada");
        }
    }

    #[test]
    fn effects_use_the_palette() {
        // Con el rótulo ya asentado, plasma pinta con los colores del tema.
        let def = &EFFECTS[fx_index("plasma").unwrap()];
        let colors = |pal: Vec<Rgb>| {
            let mut c = ctx(90, 30, Art::default_art(), 1);
            c.pal = pal;
            run(def, &mut c, 1.0 / 30.0, 5.0);
            c.g.cells.iter().map(|c| c.fg).collect::<Vec<_>>()
        };
        let a = colors(accent_palette(Rgb(0xE8, 0x11, 0x23)));
        let b = colors(accent_palette(Rgb(0x10, 0x89, 0x3E)));
        assert!(a != b);
    }

    #[test]
    fn terminal_types_the_command() {
        let def = &EFFECTS[fx_index("terminal").unwrap()];
        let mut c = ctx(90, 30, Art::default_art(), 1);
        run(def, &mut c, 1.0 / 60.0, 1.9);
        let py = (c.y0 - 3).max(0);
        let row: String = (0..c.g.cols).map(|x| c.g.cells[(py * c.g.cols + x) as usize].ch).collect();
        assert!(row.contains("❯ whoami"), "{row:?}");
    }

    #[test]
    fn clock_shows_time_and_date() {
        let def = &EFFECTS[fx_index("clock").unwrap()];
        let frame = |hour, minute, lang| {
            let mut c = ctx(90, 40, Art::default_art(), 1);
            c.now.hour = hour;
            c.now.minute = minute;
            c.lang = lang;
            run(def, &mut c, 1.0 / 30.0, 3.0);
            let text: String = c.g.cells.chunks(c.g.cols as usize).map(|r| r.iter().map(|c| c.ch).collect::<String>() + "\n").collect();
            (c.g.cells.clone(), text)
        };
        let (a, ta) = frame(21, 47, Lang::Es);
        let (b, _) = frame(9, 5, Lang::Es);
        assert!(a != b, "la hora no cambia el dibujo");
        assert!(ta.contains("lunes, 9 de noviembre de 2026"), "{ta}");
        let (_, te) = frame(21, 47, Lang::En);
        assert!(te.contains("Monday, November 9, 2026"), "{te}");
        // Valores fuera de rango no rompen nada.
        for (h, m) in [(-1, -1), (99, 99), (i32::MAX, i32::MIN)] {
            frame(h, m, Lang::En);
        }
    }

    #[test]
    fn long_date_formats_and_clamps() {
        use classic::long_date;
        let mut n = WallTime { year: 2026, month: 1, day: 31, weekday: 6, hour: 0, minute: 0, second: 0 };
        assert_eq!(long_date(Lang::Es, &n), "domingo, 31 de enero de 2026");
        assert_eq!(long_date(Lang::En, &n), "Sunday, January 31, 2026");
        n.month = 12;
        n.weekday = 2;
        assert_eq!(long_date(Lang::Es, &n), "miércoles, 31 de diciembre de 2026");
        // Fuera de rango: se ajusta en vez de entrar en pánico.
        for (m, wd) in [(0, -1), (13, 7), (i32::MIN, i32::MIN), (i32::MAX, i32::MAX)] {
            n.month = m;
            n.weekday = wd;
            assert!(!long_date(Lang::Es, &n).is_empty());
            assert!(!long_date(Lang::En, &n).is_empty());
        }
        assert_eq!(long_date(Lang::En, &WallTime::default()), "Monday, January 0, 0");
    }
}
