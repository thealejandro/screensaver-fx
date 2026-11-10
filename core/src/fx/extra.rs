//! Más efectos de una pantalla: remolino, gota, luciérnagas, aurora, láser, ecualizador, mosaico,
//! tormenta, persiana y espiral.

use super::{clamp01, ease_in_out_cubic, ease_out_cubic, flash, settle, shine, Ctx, Effect};
use crate::color::{gradient, mix, scale, Rgb, WHITE};
use crate::rng::hf;
use core::f32::consts::TAU;

/// Celda de la rejilla donde va la letra `(x, y)` del rótulo.
fn target(c: &Ctx, x: i32, y: i32) -> (f32, f32) {
    ((c.x0 + x) as f32, (c.y0 + y) as f32)
}

/// Centro del rótulo en la rejilla.
fn art_center(c: &Ctx) -> (f32, f32) {
    (c.x0 as f32 + c.art.w as f32 / 2.0, c.y0 as f32 + c.art.h as f32 / 2.0)
}

/// Distancia en celdas "cuadradas": una fila mide lo que dos columnas.
fn dist(dx: f32, dy: f32) -> f32 {
    dx.hypot(dy * 2.0)
}

fn smooth(v: f32) -> f32 {
    let v = clamp01(v);
    v * v * (3.0 - 2.0 * v)
}

// ── Remolino ────────────────────────────────────────────────────────────────

/// Las letras giran en espiral alrededor del centro y se van posando en su sitio.
struct Vortex {
    /// (ángulo inicial, radio inicial, salida, llegada) por letra.
    p: Vec<(f32, f32, f32, f32)>,
    spin: f32,
}

pub fn vortex(c: &mut Ctx) -> Box<dyn Effect> {
    let reach = (c.g.cols as f32).max(c.g.rows as f32 * 2.0) * 0.55;
    let p = (0..c.art.cells.len())
        .map(|_| {
            let go = c.rng.range(0.0, 1.4);
            (c.rng.range(0.0, TAU), reach * c.rng.range(0.55, 1.1), go, go + c.rng.range(3.6, 5.6))
        })
        .collect();
    let spin = if c.rng.chance(0.5) { 1.0 } else { -1.0 };
    Box::new(Vortex { p, spin })
}

impl Effect for Vortex {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (cx, cy) = art_center(c);
        for (i, &(x, y, ch)) in c.art.cells.iter().enumerate() {
            let (a0, r0, go, land) = self.p[i];
            if t < go {
                continue;
            }
            let base = settle(&c.pal, &c.art, x, y, t);
            let (tx, ty) = target(c, x, y);
            if t >= land {
                let col = shine(flash(base, t - land, 0.45), &c.art, x, y, t, 8.0, 9.8);
                c.g.put(tx as i32, ty as i32, ch, col);
                continue;
            }
            let k = ease_in_out_cubic((t - go) / (land - go));
            // Del círculo exterior al sitio de la letra: el radio se encoge y el giro se frena.
            let a = a0 + self.spin * (1.0 - k) * 2.5 * TAU;
            let r = r0 * (1.0 - k);
            let (ox, oy) = (cx + a.cos() * r, cy + a.sin() * r * 0.5);
            let (px, py) = (ox + (tx - cx) * k, oy + (ty - cy) * k);
            let col = scale(mix(base, WHITE, 0.3 * (1.0 - k)), 0.35 + 0.65 * k);
            c.g.put(px.round() as i32, py.round() as i32, ch, col);
        }
    }
}

// ── Gota ────────────────────────────────────────────────────────────────────

const RING: [char; 3] = ['░', '▒', '▓'];
const DROP_FALL: f32 = 1.1;
const RING_SPEED: f32 = 26.0;
const RING_GAP: f32 = 7.0;
const RINGS: usize = 3;

/// Cae una gota en el centro y las ondas que levanta van dejando el rótulo al pasar.
struct Ripple;

pub fn ripple(_c: &mut Ctx) -> Box<dyn Effect> {
    Box::new(Ripple)
}

impl Effect for Ripple {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (cx, cy) = art_center(c);
        if t < DROP_FALL {
            // La gota: una cabeza clara y una estela que se apaga hacia arriba.
            let y = cy * ease_in_out_cubic(t / DROP_FALL).powf(1.4);
            for k in 0..4 {
                let col = scale(mix(gradient(&c.pal, 0.5), WHITE, 0.6), 1.0 - k as f32 * 0.25);
                c.g.put(cx as i32, y as i32 - k, if k == 0 { '█' } else { '│' }, col);
            }
            return;
        }
        let r = (t - DROP_FALL) * RING_SPEED;
        let far = dist(c.g.cols as f32, c.g.rows as f32);
        // Las ondas recorren toda la pantalla y se apagan a medida que se alejan.
        if r < far + RING_GAP * RINGS as f32 {
            for gy in 0..c.g.rows {
                for gx in 0..c.g.cols {
                    let d = dist(gx as f32 + 0.5 - cx, gy as f32 + 0.5 - cy);
                    let mut v = 0.0f32;
                    for j in 0..RINGS {
                        let rj = r - j as f32 * RING_GAP;
                        if rj > 0.0 {
                            v = v.max((1.0 - (d - rj).abs() / 1.6) * (1.0 - j as f32 * 0.3));
                        }
                    }
                    if v > 0.0 {
                        let v = v * (1.0 - d / far).max(0.0);
                        let ch = RING[((v * 2.99) as usize).min(2)];
                        c.g.put(gx, gy, ch, scale(gradient(&c.pal, d / far + t * 0.05), 0.25 + 0.6 * v));
                    }
                }
            }
        }
        for &(x, y, ch) in &c.art.cells {
            let (tx, ty) = target(c, x, y);
            let d = dist(tx + 0.5 - cx, ty + 0.5 - cy);
            let hit = DROP_FALL + d / RING_SPEED;
            if t < hit {
                continue;
            }
            // Cada onda que pasa por la letra la aviva un poco.
            let mut lift = 0.0f32;
            for j in 1..RINGS {
                lift = lift.max(1.0 - ((d - (r - j as f32 * RING_GAP)).abs() / 2.0).min(1.0));
            }
            let base = settle(&c.pal, &c.art, x, y, t);
            let col = mix(flash(base, t - hit, 0.5), WHITE, lift * 0.35);
            c.g.put(tx as i32, ty as i32, ch, shine(col, &c.art, x, y, t, 7.5, 9.3));
        }
    }
}

// ── Luciérnagas ─────────────────────────────────────────────────────────────

const FLIGHT: f32 = 1.6;

/// Puntos de luz que revolotean por la pantalla y luego vuelan a formar el rótulo.
struct Fireflies {
    /// (x, y, fase, velocidad, salida) por letra.
    p: Vec<(f32, f32, f32, f32, f32)>,
}

pub fn fireflies(c: &mut Ctx) -> Box<dyn Effect> {
    let (cols, rows) = (c.g.cols.max(1) as f32, c.g.rows.max(1) as f32);
    let p = (0..c.art.cells.len())
        .map(|_| (c.rng.range(0.0, cols), c.rng.range(0.0, rows), c.rng.range(0.0, TAU), c.rng.range(0.5, 1.3), c.rng.range(3.5, 6.5)))
        .collect();
    Box::new(Fireflies { p })
}

impl Fireflies {
    /// Dónde revolotea la luciérnaga `i` en el instante `t`.
    fn wander(&self, i: usize, t: f32) -> (f32, f32) {
        let (x, y, ph, sp, _) = self.p[i];
        (x + (t * sp + ph).sin() * 5.0 + (t * sp * 0.37 + ph * 2.0).sin() * 3.0, y + (t * sp * 0.8 + ph * 1.7).cos() * 2.0)
    }
}

impl Effect for Fireflies {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let fade = clamp01(t / 1.0);
        for (i, &(x, y, ch)) in c.art.cells.iter().enumerate() {
            let (_, _, ph, sp, go) = self.p[i];
            let base = settle(&c.pal, &c.art, x, y, t);
            let (tx, ty) = target(c, x, y);
            let land = go + FLIGHT;
            if t >= land {
                // Ya en su sitio: la luz se queda latiendo un rato antes de asentarse.
                let pulse = (1.0 - clamp01((t - land) / 3.0)) * (0.5 + 0.5 * (t * 4.0 + ph).sin());
                let col = mix(flash(base, t - land, 0.4), WHITE, pulse * 0.25);
                c.g.put(tx as i32, ty as i32, ch, shine(col, &c.art, x, y, t, 10.0, 11.8));
                continue;
            }
            let (wx, wy) = self.wander(i, t);
            let k = ease_in_out_cubic((t - go) / FLIGHT);
            let (px, py) = (wx + (tx - wx) * k, wy + (ty - wy) * k);
            let glow = smooth(0.5 + 0.5 * (t * sp * 3.0 + ph).sin());
            let glow = (glow * (1.0 - k) + k) * fade;
            let mark = if k > 0.0 { '•' } else if glow > 0.6 { '*' } else { '·' };
            let col = scale(mix(base, WHITE, 0.35), 0.2 + 0.8 * glow);
            c.g.put(px.round() as i32, py.round() as i32, mark, col);
        }
    }
}

// ── Aurora ──────────────────────────────────────────────────────────────────

const VEIL: [char; 4] = [' ', '░', '▒', '▓'];

/// Cortinas de aurora que ondean detrás mientras el rótulo aparece poco a poco.
struct Aurora {
    /// Retraso de aparición de cada letra.
    p: Vec<f32>,
    seed: f32,
    /// Por columna y fotograma: borde inferior, brillo de los rayos y color.
    cols: Vec<(f32, f32, Rgb)>,
}

pub fn aurora(c: &mut Ctx) -> Box<dyn Effect> {
    let p = (0..c.art.cells.len()).map(|_| c.rng.range(0.0, 2.2)).collect();
    Box::new(Aurora { p, seed: c.rng.range(0.0, 100.0), cols: Vec::new() })
}

impl Aurora {
    /// La forma de la cortina solo depende de la columna: se calcula una vez por fotograma.
    fn columns(&mut self, c: &Ctx, t: f32) {
        let (s, rows, w) = (self.seed, c.g.rows.max(1) as f32, c.g.cols.max(1) as f32);
        self.cols.clear();
        self.cols.extend((0..c.g.cols).map(|gx| {
            let x = gx as f32;
            // Borde inferior de la cortina: ondea despacio de lado a lado.
            let edge = rows * 0.42 + (x * 0.05 + t * 0.45 + s).sin() * rows * 0.08 + (x * 0.019 - t * 0.27 + s * 0.7).sin() * rows * 0.06;
            // Rayos verticales finos que se mecen, y zonas más y menos intensas a lo largo.
            let ray = 0.5 + 0.5 * (x * 0.75 + (x * 0.11 + t * 0.7 + s).sin() * 2.2 + t * 0.5).sin();
            let along = 0.45 + 0.55 * (0.5 + 0.5 * (x * 0.035 - t * 0.22 + s * 1.3).sin());
            (edge, (0.3 + 0.7 * ray * ray) * along, gradient(&c.pal, x / w * 0.6 + t * 0.04))
        }));
    }

    /// Intensidad de la aurora en una celda (0..1). Lo más brillante es el borde; hacia arriba la
    /// luz se desvanece y por debajo se corta enseguida.
    fn veil(&self, gx: i32, y: f32, rows: f32) -> f32 {
        let Some(&(edge, light, _)) = self.cols.get(gx.max(0) as usize) else { return 0.0 };
        let v = if y > edge { 1.0 - (y - edge) / 1.5 } else { 1.0 - (edge - y) / (rows * 0.38) };
        smooth(v) * light
    }
}

impl Effect for Aurora {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let rows = c.g.rows.max(1) as f32;
        let fade = clamp01(t / 2.0);
        self.columns(c, t);
        if fade > 0.0 {
            for gx in 0..c.g.cols {
                let (edge, _, col) = self.cols[gx as usize];
                // Solo las filas por las que pasa la cortina.
                let (top, bottom) = ((edge - rows * 0.38).floor().max(0.0) as i32, ((edge + 1.5).ceil() as i32).min(c.g.rows - 1));
                for gy in top..=bottom {
                    let v = self.veil(gx, gy as f32, rows) * fade;
                    if v > 0.08 {
                        c.g.put(gx, gy, VEIL[((v * 3.99) as usize).min(3)], scale(col, 0.18 + 0.32 * v));
                    }
                }
            }
        }
        for (i, &(x, y, ch)) in c.art.cells.iter().enumerate() {
            let appear = 2.0 + self.p[i] + x as f32 / c.art.w.max(1) as f32 * 1.5;
            let k = ease_out_cubic((t - appear) / 1.2);
            if k <= 0.0 {
                continue;
            }
            let (tx, ty) = target(c, x, y);
            // La letra se tiñe con la luz de la aurora que tiene detrás.
            let (v, tint) = (self.veil(tx as i32, ty, rows), self.cols.get(tx as usize).map_or(WHITE, |p| p.2));
            let col = mix(settle(&c.pal, &c.art, x, y, t), tint, v * 0.3);
            c.g.put(tx as i32, ty as i32, ch, shine(scale(col, k), &c.art, x, y, t, 9.0, 10.8));
        }
    }
}

// ── Láser ───────────────────────────────────────────────────────────────────

const ENGRAVE_START: f32 = 0.8;
const ENGRAVE_DUR: f32 = 6.0;
const SPARKS: u32 = 8;

/// Un rayo baja desde lo alto de la pantalla y graba el rótulo fila a fila, soltando chispas.
struct Laser {
    /// Momento en que se graba cada letra.
    at: Vec<f32>,
    /// Letras en el orden en que se graban.
    order: Vec<usize>,
}

pub fn laser(c: &mut Ctx) -> Box<dyn Effect> {
    // Fila a fila y en zigzag, como una grabadora.
    let mut order: Vec<usize> = (0..c.art.cells.len()).collect();
    order.sort_by_key(|&i| {
        let (x, y, _) = c.art.cells[i];
        (y, if y % 2 == 0 { x } else { -x })
    });
    let n = order.len().max(1) as f32;
    let mut at = vec![0.0; order.len()];
    for (k, &i) in order.iter().enumerate() {
        at[i] = ENGRAVE_START + ENGRAVE_DUR * k as f32 / n;
    }
    Box::new(Laser { at, order })
}

impl Effect for Laser {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        // Antes de grabarse, las letras se intuyen muy tenues, como el boceto en la plancha.
        let guide = clamp01((t - 0.2) / 0.6) * 0.12;
        for (i, &(x, y, ch)) in c.art.cells.iter().enumerate() {
            let base = settle(&c.pal, &c.art, x, y, t);
            let since = t - self.at[i];
            let col = if since >= 0.0 {
                // Recién grabada está al rojo blanco y se enfría hasta su color.
                shine(flash(base, since, 0.9), &c.art, x, y, t, 8.6, 10.4)
            } else {
                scale(base, guide)
            };
            c.g.put(c.x0 + x, c.y0 + y, ch, col);
        }
        let p = (t - ENGRAVE_START) / ENGRAVE_DUR;
        if !(0.0..1.0).contains(&p) || self.order.is_empty() {
            return;
        }
        // Rayo desde el emisor, arriba al centro, hasta la letra que se graba ahora.
        let i = self.order[((p * self.order.len() as f32) as usize).min(self.order.len() - 1)];
        let (x, y, _) = c.art.cells[i];
        let (tx, ty) = target(c, x, y);
        let (ex, ey) = (c.g.cols as f32 / 2.0, -1.0);
        let (dx, dy) = (tx - ex, ty - ey);
        let ch = if dx.abs() < dy.abs() * 0.5 {
            '│'
        } else if dy.abs() * 2.0 < dx.abs() * 0.5 {
            '─'
        } else if dx > 0.0 {
            '\\'
        } else {
            '/'
        };
        let beam = mix(gradient(&c.pal, 0.8), WHITE, 0.65);
        let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as i32;
        for s in 0..steps {
            let k = s as f32 / steps as f32;
            c.g.put((ex + dx * k).round() as i32, (ey + dy * k).round() as i32, ch, scale(beam, 0.6 + 0.4 * k));
        }
        c.g.put(tx as i32, ty as i32, '█', WHITE);
        for k in 0..SPARKS {
            // Cada chispa nace en el punto de grabado, sale despedida hacia un lado y cae.
            let life = t * 2.5 + k as f32 / SPARKS as f32;
            let (age, gen) = (life.fract(), life.floor() as u32);
            let vx = (hf(k, gen, 7) - 0.5) * 12.0;
            let (sx, sy) = (tx + vx * age, ty - 1.5 * age + 4.0 * age * age);
            let col = scale(mix(WHITE, gradient(&c.pal, 0.9), age), 1.0 - age * 0.7);
            c.g.put(sx.round() as i32, sy.round() as i32, if age < 0.35 { '*' } else { '·' }, col);
        }
    }
}

// ── Ecualizador ─────────────────────────────────────────────────────────────

const EQ_BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
const EQ_LOCK: f32 = 5.0;
const EQ_SWEEP: f32 = 2.0;

/// Barras de ecualizador que saltan al ritmo y, de izquierda a derecha, bajan y dejan el rótulo.
struct Equalizer {
    /// (frecuencia, fase) de cada columna del rótulo.
    p: Vec<(f32, f32)>,
}

pub fn equalizer(c: &mut Ctx) -> Box<dyn Effect> {
    let p = (0..c.art.w.max(1)).map(|_| (c.rng.range(3.0, 7.0), c.rng.range(0.0, TAU))).collect();
    Box::new(Equalizer { p })
}

impl Equalizer {
    /// Altura (en filas, con fracción) de la barra de la columna `x`, entre 0 y `max`.
    fn level(&self, x: i32, t: f32, max: f32) -> f32 {
        let (f, ph) = self.p[x as usize];
        // Un golpe de bombo común a todas y una parte propia de cada columna.
        let beat = (1.0 - (t * 2.2).fract()).powi(2);
        let own = 0.5 + 0.5 * (t * f + ph).sin();
        max * (0.12 + 0.88 * clamp01(0.6 * own + 0.4 * beat))
    }

    fn lock(x: i32, w: f32) -> f32 {
        EQ_LOCK + EQ_SWEEP * x as f32 / w
    }
}

impl Effect for Equalizer {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (w, h) = (c.art.w.max(1) as f32, c.art.h.max(1) as f32);
        let fade = clamp01(t / 0.6);
        let bottom = c.y0 + c.art.h - 1;
        let max = h + 4.0;
        for x in 0..c.art.w {
            let lock = Self::lock(x, w);
            // Barras de dos columnas con un hueco entre ellas.
            if t >= lock || x % 3 == 2 {
                continue;
            }
            // Al acercarse su momento, la barra se encoge hasta desaparecer.
            let lv = self.level(x, t, max) * (1.0 - smooth((t - (lock - 0.8)) / 0.8));
            let full = lv as i32;
            for k in 0..=full {
                let part = if k < full { 1.0 } else { lv.fract() };
                if part < 0.07 {
                    continue;
                }
                let ch = EQ_BARS[((part * 8.0) as usize).min(7)];
                let col = gradient(&c.pal, k as f32 / max);
                c.g.put(c.x0 + x, bottom - k, ch, scale(col, (0.45 + 0.55 * k as f32 / max) * fade));
            }
        }
        for &(x, y, ch) in &c.art.cells {
            let lock = Self::lock(x, w);
            // Las letras crecen desde abajo, como la barra que se va.
            let rise = clamp01((t - (lock - 0.25)) / 0.35);
            if rise <= 0.0 || (h - 1.0 - y as f32) / h > rise {
                continue;
            }
            let col = shine(flash(settle(&c.pal, &c.art, x, y, t), t - lock, 0.5), &c.art, x, y, t, 9.0, 10.8);
            c.g.put(c.x0 + x, c.y0 + y, ch, col);
        }
    }
}

// ── Mosaico ─────────────────────────────────────────────────────────────────

/// Alto de los bloques en cada paso (de ancho, el doble, para que se vean cuadrados).
const TILES: [i32; 6] = [8, 6, 4, 3, 2, 1];
const TILE_START: f32 = 0.4;
const TILE_STEP: f32 = 0.9;

/// El rótulo aparece como un mosaico de bloques grandes que se van afinando hasta las letras.
struct Mosaic {
    /// Celdas del rótulo con letra (w x h).
    mask: Vec<bool>,
}

pub fn mosaic(c: &mut Ctx) -> Box<dyn Effect> {
    let w = c.art.w.max(0) as usize;
    let mut mask = vec![false; w * c.art.h.max(0) as usize];
    for &(x, y, _) in &c.art.cells {
        mask[y as usize * w + x as usize] = true;
    }
    Box::new(Mosaic { mask })
}

impl Effect for Mosaic {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        if t < TILE_START {
            return;
        }
        let step = ((t - TILE_START) / TILE_STEP) as usize;
        if step + 1 >= TILES.len() {
            // Último paso: las letras de verdad.
            let since = t - (TILE_START + TILE_STEP * (TILES.len() - 1) as f32);
            for &(x, y, ch) in &c.art.cells {
                let col = shine(flash(settle(&c.pal, &c.art, x, y, t), since, 0.5), &c.art, x, y, t, 8.5, 10.3);
                c.g.put(c.x0 + x, c.y0 + y, ch, col);
            }
            return;
        }
        let (bh, bw) = (TILES[step], TILES[step] * 2);
        // Cada paso entra con un golpe de brillo que se apaga enseguida.
        let pulse = 1.0 - clamp01((t - TILE_START - TILE_STEP * step as f32) / 0.35) * 0.3;
        let (w, h) = (c.art.w, c.art.h);
        for by in (0..h).step_by(bh as usize) {
            for bx in (0..w).step_by(bw as usize) {
                let (x1, y1) = ((bx + bw).min(w), (by + bh).min(h));
                let on: i32 = (by..y1).map(|y| (bx..x1).filter(|&x| self.mask[(y * w + x) as usize]).count() as i32).sum();
                let cover = on as f32 / ((x1 - bx) * (y1 - by)) as f32;
                if cover <= 0.3 {
                    continue;
                }
                let ch = if cover > 0.6 { '█' } else { '▓' };
                let col = scale(settle(&c.pal, &c.art, (bx + x1) / 2, (by + y1) / 2, t), pulse * (0.55 + 0.45 * cover));
                for y in by..y1 {
                    for x in bx..x1 {
                        c.g.put(c.x0 + x, c.y0 + y, ch, col);
                    }
                }
            }
        }
    }
}

// ── Tormenta ────────────────────────────────────────────────────────────────

/// Cuándo cae cada rayo; cada uno enciende un tramo del rótulo, de izquierda a derecha.
const STRIKES: [f32; 5] = [1.0, 2.2, 3.1, 4.3, 5.4];
const BOLT_LIFE: f32 = 0.35;

/// Lluvia en la oscuridad; cada rayo que cae enciende un tramo del rótulo.
struct Storm {
    seed: u32,
    /// Desvío del punto de salida de cada rayo, en fracción del ancho de la pantalla.
    from: [f32; STRIKES.len()],
}

pub fn storm(c: &mut Ctx) -> Box<dyn Effect> {
    let seed = c.rng.next_u64() as u32;
    let from = core::array::from_fn(|_| c.rng.range(-0.15, 0.15));
    Box::new(Storm { seed, from })
}

impl Storm {
    /// Tramo del rótulo al que pertenece la columna `x`.
    fn part(x: i32, w: i32) -> usize {
        (x.max(0) as usize * STRIKES.len() / w.max(1) as usize).min(STRIKES.len() - 1)
    }
}

impl Effect for Storm {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (cols, rows) = (c.g.cols, c.g.rows);
        // Un rayo reciente ilumina la escena entera un instante.
        let glow = STRIKES.iter().filter(|&&s| t >= s).map(|&s| 1.0 - clamp01((t - s) / 0.25)).fold(0.0f32, f32::max);
        // Lluvia en diagonal que amaina al final.
        let calm = (1.0 - smooth((t - 7.0) / 2.5)) * clamp01(t / 0.5);
        if calm > 0.0 && cols > 0 && rows > 0 {
            let drops = ((cols * rows) / 45).clamp(6, 600) as u32;
            let rain = scale(gradient(&c.pal, 0.25), (0.22 + 0.5 * glow) * calm);
            for k in 0..drops {
                let p = (t * (0.9 + 0.6 * hf(k, self.seed, 1)) + hf(k, self.seed, 2)).fract();
                let y = p * (rows + 3) as f32 - 2.0;
                let x = (hf(k, self.seed, 3) * cols as f32 + y * 0.5).rem_euclid(cols as f32);
                c.g.put(x as i32, y.floor() as i32, '\\', rain);
            }
        }
        let w = c.art.w.max(1);
        for (i, &s) in STRIKES.iter().enumerate() {
            let age = t - s;
            // Parpadea mientras dura.
            if !(0.0..BOLT_LIFE).contains(&age) || (age * 30.0) as i32 % 3 == 1 {
                continue;
            }
            let tx = c.x0 as f32 + (i as f32 + 0.5) / STRIKES.len() as f32 * w as f32;
            let col = mix(WHITE, gradient(&c.pal, 0.7), age / BOLT_LIFE);
            // Del cielo a lo alto de su tramo, en zigzag.
            let mut x = tx + self.from[i] * cols as f32;
            for y in 0..c.y0 {
                let left = (c.y0 - y) as f32;
                let step = ((tx - x) / left + (hf(i as u32, y as u32, self.seed) - 0.5) * 2.6).clamp(-2.0, 2.0);
                // Núcleo de bloque con un halo tenue a los lados; en los saltos largos rellena el hueco.
                let (a, b) = (x.round() as i32, (x + step).round() as i32);
                let (lo, hi) = (a.min(b), a.max(b));
                let halo = scale(col, 0.35);
                c.g.put(lo - 1, y, '░', halo);
                c.g.put(hi + 1, y, '░', halo);
                for gx in lo..=hi {
                    c.g.put(gx, y, '█', col);
                }
                x += step;
            }
        }
        for &(x, y, ch) in &c.art.cells {
            let hit = STRIKES[Self::part(x, w)];
            let base = settle(&c.pal, &c.art, x, y, t);
            if t < hit {
                // Los relámpagos dejan ver un momento la silueta de lo que falta.
                if glow > 0.0 {
                    c.g.put(c.x0 + x, c.y0 + y, ch, scale(base, 0.3 * glow));
                }
                continue;
            }
            let col = mix(flash(base, t - hit, 0.7), WHITE, glow * 0.25);
            c.g.put(c.x0 + x, c.y0 + y, ch, shine(col, &c.art, x, y, t, 8.6, 10.4));
        }
    }
}

// ── Persiana ────────────────────────────────────────────────────────────────

const BLIND_START: f32 = 0.5;
const BLIND_SWEEP: f32 = 2.6;
const BLIND_SLANT: f32 = 0.5;
const BLIND_TURN: f32 = 0.7;

/// Una persiana cubre la pantalla y sus lamas giran de arriba abajo hasta dejar ver el rótulo.
struct Blinds;

pub fn blinds(_c: &mut Ctx) -> Box<dyn Effect> {
    Box::new(Blinds)
}

impl Blinds {
    /// Cuándo termina de abrirse la lama en la celda `(x, y)` de la pantalla.
    fn opened(x: i32, y: i32, cols: i32, rows: i32) -> f32 {
        BLIND_START + BLIND_SWEEP * y as f32 / rows.max(1) as f32 + BLIND_SLANT * x as f32 / cols.max(1) as f32 + BLIND_TURN
    }
}

impl Effect for Blinds {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (cols, rows) = (c.g.cols, c.g.rows);
        if t < BLIND_START + BLIND_SWEEP + BLIND_SLANT + BLIND_TURN {
            let fade = clamp01(t / 0.4);
            for y in 0..rows {
                for x in 0..cols {
                    let o = clamp01(1.0 - (Self::opened(x, y, cols, rows) - t) / BLIND_TURN);
                    if o >= 1.0 {
                        continue;
                    }
                    // Al girar, la lama se ve cada vez más fina y le da la luz a mitad de giro.
                    let ch = EQ_BARS[7 - ((o * 8.0) as usize).min(7)];
                    let g = gradient(&c.pal, x as f32 / cols as f32 * 0.6 + y as f32 / rows as f32 * 0.4);
                    let shade = if y % 2 == 0 { 0.34 } else { 0.26 };
                    c.g.put(x, y, ch, scale(g, (shade + 1.6 * o * (1.0 - o)) * fade));
                }
            }
        }
        for &(x, y, ch) in &c.art.cells {
            let since = t - Self::opened(c.x0 + x, c.y0 + y, cols, rows);
            if since < 0.0 {
                continue;
            }
            let col = shine(flash(settle(&c.pal, &c.art, x, y, t), since, 0.5), &c.art, x, y, t, 7.5, 9.3);
            c.g.put(c.x0 + x, c.y0 + y, ch, col);
        }
    }
}

// ── Espiral ─────────────────────────────────────────────────────────────────

const SPIRAL_CH: [char; 4] = ['·', '░', '▒', '▓'];
/// Giro de la espiral en radianes por segundo.
const SPIRAL_SPIN: f32 = 3.0;
const SPIRAL_REVEAL: f32 = 1.2;
const SPIRAL_REVEAL_DUR: f32 = 3.5;

/// Los brazos de una espiral giran hacia el centro y encienden las letras al pasar por encima.
struct Spiral {
    arms: f32,
    dir: f32,
    twist: f32,
}

pub fn spiral(c: &mut Ctx) -> Box<dyn Effect> {
    let arms = [2.0, 3.0, 4.0][c.rng.below(3)];
    let dir = if c.rng.chance(0.5) { 1.0 } else { -1.0 };
    let twist = c.rng.range(0.22, 0.34);
    Box::new(Spiral { arms, dir, twist })
}

impl Spiral {
    /// Fase de la espiral en un punto relativo al centro: los brazos están donde vale 0.
    fn phase(&self, dx: f32, dy: f32) -> f32 {
        self.arms * (dy * 2.0).atan2(dx) + dist(dx, dy) * self.twist
    }
}

impl Effect for Spiral {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (cx, cy) = art_center(c);
        let fade = clamp01(t / 0.8) * (1.0 - smooth((t - 6.5) / 2.0));
        if fade > 0.0 {
            let far = dist(c.g.cols as f32, c.g.rows as f32).max(1.0);
            for gy in 0..c.g.rows {
                for gx in 0..c.g.cols {
                    let (dx, dy) = (gx as f32 + 0.5 - cx, gy as f32 + 0.5 - cy);
                    let v = clamp01(((self.phase(dx, dy) - self.dir * SPIRAL_SPIN * t).cos() - 0.55) / 0.45);
                    if v <= 0.0 {
                        continue;
                    }
                    let near = 1.0 - dist(dx, dy) / far;
                    let ch = SPIRAL_CH[((v * 3.99) as usize).min(3)];
                    let col = scale(gradient(&c.pal, near + t * 0.05), (0.2 + 0.6 * v) * (0.35 + 0.65 * near) * fade);
                    c.g.put(gx, gy, ch, col);
                }
            }
        }
        let reach = dist(c.art.w as f32 / 2.0, c.art.h as f32 / 2.0).max(1.0);
        let period = TAU / SPIRAL_SPIN;
        for &(x, y, ch) in &c.art.cells {
            let (tx, ty) = target(c, x, y);
            let (dx, dy) = (tx + 0.5 - cx, ty + 0.5 - cy);
            // De fuera hacia dentro, cada letra espera a que un brazo pase por encima.
            let from = SPIRAL_REVEAL + SPIRAL_REVEAL_DUR * (1.0 - dist(dx, dy) / reach).max(0.0);
            let phi = self.phase(dx, dy).rem_euclid(TAU);
            let first = (if self.dir > 0.0 { phi } else { (TAU - phi).rem_euclid(TAU) }) / SPIRAL_SPIN;
            let hit = first + ((from - first) / period).ceil().max(0.0) * period;
            if t < hit {
                continue;
            }
            let col = shine(flash(settle(&c.pal, &c.art, x, y, t), t - hit, 0.5), &c.art, x, y, t, 9.0, 10.8);
            c.g.put(tx as i32, ty as i32, ch, col);
        }
    }
}
