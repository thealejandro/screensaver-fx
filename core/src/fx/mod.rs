//! Efectos: cada uno dibuja la rejilla completa en cada fotograma a partir de su tiempo local.

mod classic;
mod modern;

use crate::art::Art;
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

pub static EFFECTS: [FxDef; 16] = [
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
