//! Núcleo de Screensaver FX: rejilla de celdas tipo terminal, efectos y rasterizado a un búfer
//! de píxeles. Cada plataforma solo crea una ventana, llama a `tick` y escala el búfer.

pub mod art;
pub mod color;
pub mod config;
pub mod ffi;
pub mod font;
pub mod fx;
pub mod grid;
pub mod raster;
pub mod rng;

pub use config::{Banner, Config, Font, Lang, Size, ThemeSel};

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
        // `frac` ya garantiza que caben min_cols x min_rows; el max evita perder una celda por
        // redondeo (60,9999 -> 60).
        let cols = ((w as f32 / (cw as f32 * frac)) as i32).max(min_cols);
        let rows = ((h as f32 / (ch as f32 * frac)) as i32).max(min_rows);
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
    // Rejilla virtual mínima con el aspecto de la pantalla, reducida al dibujar. El aspecto se
    // acota: una ventana de 1 px de ancho pedía un búfer de cientos de MB (queda con bandas).
    let (aw, ah) = ((w as f32).min(h as f32 * 4.0), (h as f32).min(w as f32 * 4.0));
    let mut rows = min_rows;
    let mut cols = ((rows * ch) as f32 * aw / ah / cw as f32).round() as i32;
    if cols < min_cols {
        cols = min_cols;
        rows = (((cols * cw) as f32 * ah / aw / ch as f32).round() as i32).max(min_rows);
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
    /// Sin rasterizador en una terminal: allí se pinta la rejilla tal cual.
    raster: Option<Raster>,
    phase: Phase,
    last_fx: Option<usize>,
    last_t: f64,
}

const FIRST_GAP: f64 = 0.15;
const GAP: f64 = 0.6;

impl Engine {
    /// `px_w`/`px_h`: tamaño en píxeles físicos de la pantalla o ventana.
    pub fn new(px_w: i32, px_h: i32, cfg: Config) -> Engine {
        let art = Engine::art_for(&cfg);
        let layout = compute_layout(px_w, px_h, cfg.size, (art.w + 8).max(40), art.h + 10);
        let raster = Raster::new(layout.cols, layout.rows);
        Engine::build(cfg, art, layout, Some(raster))
    }

    /// Motor para una terminal de `cols` x `rows` celdas: no rasteriza, la rejilla se lee con
    /// `grid()` y la pinta quien llama. Un rótulo que no quepa queda recortado por los bordes.
    pub fn for_terminal(cols: i32, rows: i32, cfg: Config) -> Engine {
        let (cols, rows) = (cols.clamp(1, 1000), rows.clamp(1, 500));
        let art = Engine::art_for(&cfg);
        let (bw, bh) = (cols * CW as i32, rows * CH as i32);
        let layout = Layout { cols, rows, buf_w: bw, buf_h: bh, dst_x: 0, dst_y: 0, dst_w: bw, dst_h: bh, scaled_down: false };
        Engine::build(cfg, art, layout, None)
    }

    fn art_for(cfg: &Config) -> Art {
        match cfg.banner {
            Banner::Default => None,
            Banner::Text => Art::from_text_with(&cfg.title, &cfg.subtitle, cfg.font),
            Banner::File => cfg.art.as_deref().and_then(Art::parse),
        }
        .unwrap_or_else(|| Art::default_art_with(cfg.font))
    }

    fn build(cfg: Config, art: Art, layout: Layout, raster: Option<Raster>) -> Engine {
        let mut rng = Rng::new(cfg.seed);
        let pal = match cfg.theme {
            ThemeSel::Auto => cfg.system_palette.clone().filter(|p| p.len() >= 2).unwrap_or_else(|| accent_palette(cfg.accent)),
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
            raster,
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
        self.raster.as_ref().map_or(&[], |r| &r.px)
    }

    /// Rejilla de celdas del último tick (carácter y color), para pintarla en una terminal.
    pub fn grid(&self) -> &Grid {
        &self.ctx.g
    }

    /// Rectángulo cambiado en el último tick, en píxeles del búfer.
    pub fn dirty(&self) -> Option<(u32, u32, u32, u32)> {
        self.raster.as_ref().and_then(|r| r.dirty)
    }

    pub fn current_effect(&self) -> Option<&'static str> {
        match self.phase {
            Phase::Run { .. } => self.last_fx.map(|i| EFFECTS[i].id_str()),
            Phase::Gap(_) => None,
        }
    }

    fn start_next(&mut self, t: f64) {
        let set = &self.cfg.effects;
        let mut pool: Vec<usize> = set.iter().copied().filter(|&i| Some(i) != self.last_fx).collect();
        // Un solo efecto (o el mismo repetido, `effects=neon,neon`): se repite.
        if pool.is_empty() {
            pool = set.clone();
        }
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
        if !t.is_finite() {
            // Un infinito dejaría el hueco o el efecto esperando para siempre: se ignora.
            if let Some(r) = self.raster.as_mut() {
                r.dirty = None;
            }
            return false;
        }
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
        match self.raster.as_mut() {
            Some(r) => r.update(&self.ctx.g),
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RESOLUTIONS: [(i32, i32); 14] = [
        (320, 240),
        (640, 480),
        (800, 600),
        (1024, 768),
        (1280, 720),
        (1366, 768),
        (1920, 1080),
        (2560, 1080),
        (2560, 1440),
        (3440, 1440),
        (3840, 2160),
        (5120, 2880),
        (1080, 1920),
        (152, 112), // vista previa de Configuración de Windows
    ];

    fn default_mins() -> (i32, i32) {
        let a = Art::default_art();
        ((a.w + 8).max(40), a.h + 10)
    }

    /// Invariantes que toda disposición debe cumplir.
    fn check(l: &Layout, w: i32, h: i32, mc: i32, mr: i32) {
        let what = format!("{w}x{h} min {mc}x{mr}: {l:?}");
        let (sw, sh) = (w.max(1), h.max(1));
        assert!(l.cols >= 1 && l.rows >= 1, "{what}");
        assert_eq!((l.buf_w, l.buf_h), (l.cols * CW as i32, l.rows * CH as i32), "{what}");
        // El rótulo cabe en la rejilla.
        assert!(l.cols >= mc && l.rows >= mr, "{what}: el rótulo no cabe");
        // Destino dentro de la pantalla y centrado.
        assert!(l.dst_w >= 1 && l.dst_h >= 1, "{what}");
        assert!(l.dst_x >= 0 && l.dst_y >= 0, "{what}");
        assert!(l.dst_x + l.dst_w <= sw && l.dst_y + l.dst_h <= sh, "{what}: se sale de la pantalla");
        assert!((0..=1).contains(&(sw - l.dst_w - 2 * l.dst_x)), "{what}: no centrado en x");
        assert!((0..=1).contains(&(sh - l.dst_h - 2 * l.dst_y)), "{what}: no centrado en y");
        // Mantiene la proporción (error de redondeo de 1 px).
        let err = (l.dst_w as i64 * l.buf_h as i64 - l.dst_h as i64 * l.buf_w as i64).abs();
        assert!(err <= l.buf_w.max(l.buf_h) as i64, "{what}: deforma la imagen");
        if l.scaled_down {
            assert!(l.dst_w <= l.buf_w && l.dst_h <= l.buf_h, "{what}");
            assert_eq!(l.int_scale(), None, "{what}");
        } else {
            assert!(l.dst_w >= l.buf_w && l.dst_h >= l.buf_h, "{what}");
            // Sin reducir, ocupa la pantalla salvo lo que no llega a una celda escalada.
            let k = l.dst_w as f32 / l.buf_w as f32;
            assert!((sw - l.dst_w) as f32 <= CW as f32 * k + 1.0 && (sh - l.dst_h) as f32 <= CH as f32 * k + 1.0, "{what}: deja demasiado borde");
        }
        if let Some(k) = l.int_scale() {
            assert!(k >= 1 && !l.scaled_down, "{what}");
            assert_eq!((l.dst_w, l.dst_h), (l.buf_w * k, l.buf_h * k), "{what}");
        } else if !l.scaled_down && l.dst_w % l.buf_w == 0 && l.dst_h == l.buf_h * (l.dst_w / l.buf_w) {
            panic!("{what}: escala entera no detectada");
        }
        // Búfer de tamaño razonable: nunca mayor que la pantalla o que el mínimo del rótulo
        // estirado a un aspecto 1:4 (antes, una ventana de 1x1080 pedía 257 millones de píxeles).
        let area = l.buf_w as i64 * l.buf_h as i64;
        let side = ((mc * CW as i32) as i64).max((mr * CH as i32) as i64);
        assert!(area <= (sw as i64 * sh as i64).max(4 * side * (side + CH as i64)), "{what}: búfer desmesurado ({area} px)");
    }

    #[test]
    fn layout_common_resolutions_m() {
        let (mc, mr) = default_mins();
        let cases = [
            ((1920, 1080), Layout { cols: 120, rows: 33, buf_w: 960, buf_h: 528, dst_x: 0, dst_y: 12, dst_w: 1920, dst_h: 1056, scaled_down: false }, Some(2)),
            ((3840, 2160), Layout { cols: 120, rows: 33, buf_w: 960, buf_h: 528, dst_x: 0, dst_y: 24, dst_w: 3840, dst_h: 2112, scaled_down: false }, Some(4)),
            ((1280, 720), Layout { cols: 160, rows: 45, buf_w: 1280, buf_h: 720, dst_x: 0, dst_y: 0, dst_w: 1280, dst_h: 720, scaled_down: false }, Some(1)),
            ((2560, 1440), Layout { cols: 106, rows: 30, buf_w: 848, buf_h: 480, dst_x: 8, dst_y: 0, dst_w: 2544, dst_h: 1440, scaled_down: false }, Some(3)),
            ((1080, 1920), Layout { cols: 67, rows: 60, buf_w: 536, buf_h: 960, dst_x: 4, dst_y: 0, dst_w: 1072, dst_h: 1920, scaled_down: false }, Some(2)),
        ];
        for ((w, h), want, k) in cases {
            let l = compute_layout(w, h, Size::M, mc, mr);
            assert_eq!(l, want, "{w}x{h}");
            assert_eq!(l.int_scale(), k, "{w}x{h}");
            check(&l, w, h, mc, mr);
        }
    }

    #[test]
    fn layout_small_screen_scales_down() {
        let (mc, mr) = default_mins();
        let l = compute_layout(320, 240, Size::M, mc, mr);
        check(&l, 320, 240, mc, mr);
        assert!(l.scaled_down);
        assert_eq!(l.int_scale(), None);
        assert_eq!((l.cols, l.rows), (67, 25));
        for (w, h) in [(152, 112), (100, 75), (64, 64), (10, 10), (1, 1), (2, 1), (1, 2)] {
            let l = compute_layout(w, h, Size::M, mc, mr);
            check(&l, w, h, mc, mr);
            assert!(l.scaled_down, "{w}x{h}");
        }
    }

    #[test]
    fn layout_zero_and_negative_sizes_do_not_panic() {
        let (mc, mr) = default_mins();
        for (w, h) in [(0, 0), (-1, -1), (0, 1080), (1920, 0), (i32::MIN, i32::MIN), (-1920, 1080)] {
            let l = compute_layout(w, h, Size::M, mc, mr);
            check(&l, w, h, mc, mr);
        }
    }

    #[test]
    fn layout_degenerate_aspect_keeps_buffer_small() {
        // Ventanas de 1 px de ancho o de alto (p. ej. al crearse o redimensionarse).
        let (mc, mr) = default_mins();
        for (w, h) in [(1, 1080), (1, 2160), (2, 4000), (1920, 1), (3840, 2), (10, 1000), (1000, 10)] {
            let l = compute_layout(w, h, Size::M, mc, mr);
            check(&l, w, h, mc, mr);
        }
    }

    #[test]
    fn layout_all_sizes_and_resolutions() {
        let arts = [
            Art::default_art(),
            Art::parse("X").unwrap(),
            Art::from_text(&"W".repeat(16), &"W".repeat(32)).unwrap(),
            Art::parse(&vec!["#".repeat(400); 200].join("\n")).unwrap(),
            Art::parse(&vec!["#".repeat(120); 60].join("\n")).unwrap(),
        ];
        for art in &arts {
            let (mc, mr) = ((art.w + 8).max(40), art.h + 10);
            for &(w, h) in &RESOLUTIONS {
                for size in [Size::S, Size::M, Size::L] {
                    check(&compute_layout(w, h, size, mc, mr), w, h, mc, mr);
                }
            }
        }
    }

    #[test]
    fn layout_bigger_size_means_bigger_scale() {
        let (mc, mr) = default_mins();
        for &(w, h) in &RESOLUTIONS {
            let scale = |s| {
                let l = compute_layout(w, h, s, mc, mr);
                l.dst_w as f32 / l.buf_w as f32
            };
            let (s, m, l) = (scale(Size::S), scale(Size::M), scale(Size::L));
            assert!(s <= m && m <= l, "{w}x{h}: S {s} M {m} L {l}");
        }
        let at4k = |s| compute_layout(3840, 2160, s, mc, mr).int_scale();
        assert_eq!((at4k(Size::S), at4k(Size::M), at4k(Size::L)), (Some(3), Some(4), Some(5)));
    }

    #[test]
    fn layout_sweep() {
        // Barrido denso de tamaños para pillar errores de redondeo en las tres ramas.
        let (mc, mr) = default_mins();
        let mins = [(mc, mr), (40, 11), (294, 23), (408, 210), (130, 70)];
        for &(mc, mr) in &mins {
            for w in (1..=4000).step_by(97) {
                for h in (1..=2400).step_by(61) {
                    for size in [Size::S, Size::M, Size::L] {
                        check(&compute_layout(w, h, size, mc, mr), w, h, mc, mr);
                    }
                }
            }
        }
    }

    #[test]
    fn layout_fractional_scale_for_long_banners() {
        // Rótulo largo en 4K: la escala entera que cabe (1) quedaría pequeña; se usa una fraccional.
        let art = Art::from_text("SCREENSAVER FX", "").unwrap();
        let (mc, mr) = ((art.w + 8).max(40), art.h + 10);
        let l = compute_layout(3840, 2160, Size::M, mc, mr);
        check(&l, 3840, 2160, mc, mr);
        assert!(!l.scaled_down);
        let k = l.dst_w as f32 / l.buf_w as f32;
        assert!(k > 1.25, "{l:?}");
    }

    #[test]
    fn int_scale_rules() {
        let base = Layout { cols: 10, rows: 5, buf_w: 80, buf_h: 80, dst_x: 0, dst_y: 0, dst_w: 160, dst_h: 160, scaled_down: false };
        assert_eq!(base.int_scale(), Some(2));
        assert_eq!(Layout { dst_w: 80, dst_h: 80, ..base }.int_scale(), Some(1));
        assert_eq!(Layout { dst_w: 120, dst_h: 120, ..base }.int_scale(), None);
        assert_eq!(Layout { dst_w: 160, dst_h: 150, ..base }.int_scale(), None);
        assert_eq!(Layout { scaled_down: true, ..base }.int_scale(), None);
        assert_eq!(Layout { dst_w: 40, dst_h: 40, ..base }.int_scale(), None);
        assert_eq!(Layout { buf_w: 0, ..base }.int_scale(), None);
    }
}
