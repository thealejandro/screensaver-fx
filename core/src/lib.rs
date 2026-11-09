//! Núcleo de Screensaver FX: rejilla de celdas tipo terminal, efectos y rasterizado a un búfer
//! de píxeles. Cada plataforma solo crea una ventana, llama a `tick` y escala el búfer.

pub mod art;
pub mod color;
pub mod config;
pub mod ffi;
pub mod fx;
pub mod grid;
pub mod raster;
pub mod rng;

pub use config::{Banner, Config, Lang, Size, ThemeSel};

use art::Art;
use color::{accent_palette, THEMES};
use fx::{Ctx, Effect, EFFECTS};
use grid::Grid;
use raster::{Raster, CH, CW};
use rng::Rng;

/// Hora local de pared, la da la plataforma.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WallTime {
    pub year: i32,
    /// 1..=12
    pub month: i32,
    pub day: i32,
    /// 0 = lunes .. 6 = domingo
    pub weekday: i32,
    pub hour: i32,
    pub minute: i32,
    pub second: i32,
}

/// Dónde y a qué tamaño va el búfer en la pantalla.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub cols: i32,
    pub rows: i32,
    pub buf_w: i32,
    pub buf_h: i32,
    pub dst_x: i32,
    pub dst_y: i32,
    pub dst_w: i32,
    pub dst_h: i32,
    /// Pantalla más pequeña que el rótulo (vista previa): el búfer se reduce con suavizado.
    pub scaled_down: bool,
}

impl Layout {
    /// Factor entero de ampliación, si lo es (permite copiar solo el rectángulo cambiado).
    pub fn int_scale(&self) -> Option<i32> {
        let k = self.dst_w / self.buf_w.max(1);
        (!self.scaled_down && k >= 1 && self.buf_w * k == self.dst_w && self.buf_h * k == self.dst_h).then_some(k)
    }
}

/// Escala entera para que los píxeles queden nítidos; reduce solo si el rótulo no cabe.
pub fn compute_layout(w: i32, h: i32, size: Size, min_cols: i32, min_rows: i32) -> Layout {
    let (w, h) = (w.max(1), h.max(1));
    let (cw, ch) = (CW as i32, CH as i32);
    let ideal = ((h as f32 / (ch as f32 * size.rows())).round() as i32).max(1);
    let mut k = ideal;
    while k > 1 && (h / (ch * k) < min_rows || w / (cw * k) < min_cols) {
        k -= 1;
    }
    // Rótulo largo: la escala entera que cabe quedaría muy pequeña. Se usa una escala
    // fraccional (vecino más cercano) que llena la pantalla igual.
    let frac = (ideal as f32).min(w as f32 / (cw * min_cols) as f32).min(h as f32 / (ch * min_rows) as f32);
    if k < ideal && frac >= k as f32 + 0.25 {
        let (cols, rows) = ((w as f32 / (cw as f32 * frac)) as i32, (h as f32 / (ch as f32 * frac)) as i32);
        let (bw, bh) = (cols * cw, rows * ch);
        let (dw, dh) = ((bw as f32 * frac).round() as i32, (bh as f32 * frac).round() as i32);
        return Layout { cols, rows, buf_w: bw, buf_h: bh, dst_x: (w - dw) / 2, dst_y: (h - dh) / 2, dst_w: dw, dst_h: dh, scaled_down: false };
    }
    let (cols, rows) = (w / (cw * k), h / (ch * k));
    if cols >= min_cols && rows >= min_rows {
        let (bw, bh) = (cols * cw, rows * ch);
        return Layout {
            cols,
            rows,
            buf_w: bw,
            buf_h: bh,
            dst_x: (w - bw * k) / 2,
            dst_y: (h - bh * k) / 2,
            dst_w: bw * k,
            dst_h: bh * k,
            scaled_down: false,
        };
    }
    // Rejilla virtual mínima con el aspecto de la pantalla, reducida al dibujar.
    let mut rows = min_rows;
    let mut cols = ((rows * ch) as f32 * w as f32 / h as f32 / cw as f32).round() as i32;
    if cols < min_cols {
        cols = min_cols;
        rows = (((cols * cw) as f32 * h as f32 / w as f32 / ch as f32).round() as i32).max(min_rows);
    }
    let (bw, bh) = (cols * cw, rows * ch);
    let s = (w as f32 / bw as f32).min(h as f32 / bh as f32);
    let (dw, dh) = (((bw as f32 * s).round() as i32).max(1), ((bh as f32 * s).round() as i32).max(1));
    Layout { cols, rows, buf_w: bw, buf_h: bh, dst_x: (w - dw) / 2, dst_y: (h - dh) / 2, dst_w: dw, dst_h: dh, scaled_down: true }
}

enum Phase {
    Gap(f64),
    Run { fx: Box<dyn Effect>, start: f64, dur: f64 },
}

pub struct Engine {
    cfg: Config,
    layout: Layout,
    ctx: Ctx,
    raster: Raster,
    phase: Phase,
    last_fx: Option<usize>,
    last_t: f64,
}

const FIRST_GAP: f64 = 0.15;
const GAP: f64 = 0.6;

impl Engine {
    /// `px_w`/`px_h`: tamaño en píxeles físicos de la pantalla o ventana.
    pub fn new(px_w: i32, px_h: i32, cfg: Config) -> Engine {
        let art = match cfg.banner {
            Banner::Default => None,
            Banner::Text => Art::from_text(&cfg.title, &cfg.subtitle),
            Banner::File => cfg.art.as_deref().and_then(Art::parse),
        }
        .unwrap_or_else(Art::default_art);
        let layout = compute_layout(px_w, px_h, cfg.size, (art.w + 8).max(40), art.h + 10);
        let mut rng = Rng::new(cfg.seed);
        let pal = match cfg.theme {
            ThemeSel::Auto => accent_palette(cfg.accent),
            ThemeSel::Named(i) => THEMES[i].palette(),
            ThemeSel::Random => THEMES[rng.below(THEMES.len())].palette(),
        };
        let mut ctx = Ctx {
            g: Grid::new(layout.cols, layout.rows),
            art,
            pal,
            rng,
            now: WallTime::default(),
            lang: cfg.lang,
            x0: 0,
            y0: 0,
        };
        ctx.center();
        Engine {
            raster: Raster::new(layout.cols, layout.rows),
            cfg,
            layout,
            ctx,
            phase: Phase::Gap(FIRST_GAP),
            last_fx: None,
            last_t: 0.0,
        }
    }

    pub fn layout(&self) -> Layout {
        self.layout
    }

    /// Búfer BGRX (0x00RRGGBB por píxel) de `buf_w` x `buf_h`.
    pub fn pixels(&self) -> &[u32] {
        &self.raster.px
    }

    /// Rectángulo cambiado en el último tick, en píxeles del búfer.
    pub fn dirty(&self) -> Option<(u32, u32, u32, u32)> {
        self.raster.dirty
    }

    pub fn current_effect(&self) -> Option<&'static str> {
        match self.phase {
            Phase::Run { .. } => self.last_fx.map(|i| EFFECTS[i].id_str()),
            Phase::Gap(_) => None,
        }
    }

    fn start_next(&mut self, t: f64) {
        let set = &self.cfg.effects;
        let pool: Vec<usize> = if set.len() > 1 { set.iter().copied().filter(|&i| Some(i) != self.last_fx).collect() } else { set.clone() };
        let idx = pool[self.ctx.rng.below(pool.len())];
        if self.cfg.theme == ThemeSel::Random {
            self.ctx.pal = THEMES[self.ctx.rng.below(THEMES.len())].palette();
        }
        self.ctx.center();
        let def = &EFFECTS[idx];
        let fx = (def.make)(&mut self.ctx);
        self.last_fx = Some(idx);
        self.phase = Phase::Run { fx, start: t, dur: def.secs as f64 };
    }

    /// Avanza la animación a `t` segundos (monótono, desde el inicio). Devuelve true si cambió
    /// algún píxel: si es false la plataforma puede saltarse el dibujado.
    pub fn tick(&mut self, t: f64, now: WallTime) -> bool {
        let dt = (t - self.last_t).clamp(0.0, 0.1) as f32;
        self.last_t = t;
        self.ctx.now = now;
        self.ctx.g.clear();
        if let Phase::Gap(until) = self.phase {
            if t >= until {
                self.start_next(t);
            }
        }
        if let Phase::Run { fx, start, dur } = &mut self.phase {
            let local = t - *start;
            if local >= *dur {
                self.phase = Phase::Gap(t + GAP);
            } else {
                fx.draw(local as f32, dt, &mut self.ctx);
            }
        }
        self.raster.update(&self.ctx.g)
    }
}
