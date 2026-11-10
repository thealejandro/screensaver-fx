//! Más efectos de una pantalla: remolino, gota, luciérnagas y aurora.

use super::{clamp01, ease_in_out_cubic, ease_out_cubic, flash, settle, shine, Ctx, Effect};
use crate::color::{gradient, mix, scale, Rgb, WHITE};
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
