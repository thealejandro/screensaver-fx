//! Más efectos de una pantalla: retro, burbujas y radar.

use super::extra::{art_center, dist, smooth};
use super::{clamp01, ease_out_cubic, flash, settle, shine, Ctx, Effect};
use crate::color::{gradient, mix, scale, WHITE};
use crate::rng::hf;
use core::f32::consts::{FRAC_PI_4, TAU};

// ── Retro ───────────────────────────────────────────────────────────────────

const RETRO_SCAN: f32 = 1.2;
const RETRO_SCAN_DUR: f32 = 1.4;

/// Paisaje de los ochenta: sol a rayas, suelo de rejilla que avanza y estrellas; el rótulo encima.
struct Retro {
    seed: u32,
}

pub fn retro(c: &mut Ctx) -> Box<dyn Effect> {
    Box::new(Retro { seed: c.rng.next_u64() as u32 })
}

impl Effect for Retro {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (cols, rows) = (c.g.cols, c.g.rows);
        let fade = clamp01(t / 1.0);
        // El horizonte, justo debajo del rótulo.
        let hy = (c.y0 + c.art.h + 1).min(rows - 1);
        if cols > 0 && hy > 0 {
            let vx = cols as f32 / 2.0;
            // Estrellas que titilan en el cielo.
            let stars = ((cols * hy) / 60).clamp(4, 120) as u32;
            for k in 0..stars {
                let (x, y) = ((hf(k, self.seed, 1) * cols as f32) as i32, (hf(k, self.seed, 2) * hy as f32) as i32);
                let tw = 0.5 + 0.5 * (t * (1.0 + 2.0 * hf(k, self.seed, 3)) + k as f32).sin();
                c.g.put(x, y, if tw > 0.85 { '+' } else { '·' }, scale(gradient(&c.pal, 0.9), (0.15 + 0.4 * tw) * fade));
            }
            // El sol sale del horizonte; en la mitad de abajo, medio bloque por fila hace las rayas.
            // Más ancho que el rótulo, para que asome por los lados.
            let r = (c.art.w as f32 * 0.8).max(hy as f32 * 0.8).min(cols as f32 * 0.32).min(hy as f32 * 2.0).max(1.0);
            let rise = ease_out_cubic(t / 2.5);
            let sy = hy as f32 + r * 0.5 * (1.0 - rise);
            for y in 0..hy {
                let dy = y as f32 + 0.5 - sy;
                if dy * 2.0 < -r || dy > 0.0 {
                    continue;
                }
                let v = -dy * 2.0 / r;
                let ch = if v < 0.55 { '▄' } else { '█' };
                let half = (r * r - dy * dy * 4.0).max(0.0).sqrt();
                let col = scale(gradient(&c.pal, 0.15 + 0.7 * (1.0 - v)), 0.42 * fade);
                for x in (vx - half).round() as i32..(vx + half).round() as i32 {
                    c.g.put(x, y, ch, col);
                }
            }
            // Suelo: líneas horizontales que se acercan y líneas que huyen hacia el centro.
            let depth = (rows - hy).max(1) as f32;
            let grid = gradient(&c.pal, 0.8);
            for n in 0..16 {
                let z = n as f32 + 1.0 - (t * 0.8).rem_euclid(1.0);
                let y = hy as f32 + depth * 0.9 / z;
                let near = (y - hy as f32) / depth;
                if y.round() as i32 > hy {
                    for x in 0..cols {
                        c.g.put(x, y.round() as i32, '─', scale(grid, (0.2 + 0.6 * near) * fade));
                    }
                }
            }
            // Solo las líneas de fuga que no van casi tumbadas; las de los lados, más tenues.
            const SPACING: f32 = 8.0;
            const FLAT: f32 = 2.5;
            let lanes = (FLAT * depth / SPACING) as i32;
            for y in hy + 1..rows {
                let d = (y - hy) as f32 / depth;
                for k in -lanes..=lanes {
                    let slope = k as f32 * SPACING / depth;
                    let ch = if slope.abs() < 0.6 {
                        '│'
                    } else if slope > 0.0 {
                        '╲'
                    } else {
                        '╱'
                    };
                    let x0 = vx + k as f32 * SPACING * d;
                    let col = scale(grid, (0.2 + 0.6 * d) * (1.0 - slope.abs() / (FLAT + 1.0)) * fade);
                    // Las más tumbadas ocupan dos celdas por fila.
                    let n = slope.abs().max(1.0) as i32;
                    for j in 0..n {
                        c.g.put((x0 + slope.signum() * j as f32).round() as i32, y, ch, col);
                    }
                }
            }
            for x in 0..cols {
                c.g.put(x, hy, '─', scale(mix(grid, WHITE, 0.3), 0.9 * fade));
            }
        }
        // El rótulo se dibuja de arriba abajo, como un barrido de pantalla antigua.
        let h = c.art.h.max(1) as f32;
        for &(x, y, ch) in &c.art.cells {
            let at = RETRO_SCAN + RETRO_SCAN_DUR * y as f32 / h;
            if t < at {
                continue;
            }
            let col = shine(flash(settle(&c.pal, &c.art, x, y, t), t - at, 0.6), &c.art, x, y, t, 10.0, 11.8);
            c.g.put(c.x0 + x, c.y0 + y, ch, col);
        }
    }
}

// ── Burbujas ────────────────────────────────────────────────────────────────

const BUB_W: i32 = 10;
const BUB_H: i32 = 5;
const POP: f32 = 0.35;
const RIM: usize = 36;

/// Las letras suben desde abajo dentro de burbujas que se mecen y revientan al llegar a su sitio.
struct Bubbles {
    /// (salida, duración de la subida, fase del vaivén, lleva letras) por bloque del rótulo.
    b: Vec<(f32, f32, f32, bool)>,
    /// Bloques por fila.
    nbx: i32,
    seed: u32,
}

pub fn bubbles(c: &mut Ctx) -> Box<dyn Effect> {
    let nbx = (c.art.w.max(0) + BUB_W - 1) / BUB_W;
    let nby = (c.art.h.max(0) + BUB_H - 1) / BUB_H;
    let mut b: Vec<_> = (0..nbx * nby).map(|_| (c.rng.range(0.2, 5.0), c.rng.range(2.4, 3.6), c.rng.range(0.0, TAU), false)).collect();
    for &(x, y, _) in &c.art.cells {
        b[((y / BUB_H) * nbx + x / BUB_W) as usize].3 = true;
    }
    Box::new(Bubbles { b, nbx, seed: c.rng.next_u64() as u32 })
}

impl Bubbles {
    /// Centro del bloque `i` en la rejilla (en su sitio final).
    fn home(&self, i: usize, c: &Ctx) -> (f32, f32) {
        let (bx, by) = (i as i32 % self.nbx * BUB_W, i as i32 / self.nbx * BUB_H);
        let (w, h) = ((c.art.w - bx).min(BUB_W), (c.art.h - by).min(BUB_H));
        ((c.x0 + bx) as f32 + w as f32 / 2.0, (c.y0 + by) as f32 + h as f32 / 2.0)
    }

    /// Cuánto se desplaza ahora la burbuja `i` respecto a su sitio, o None si ya llegó.
    fn offset(&self, i: usize, t: f32, c: &Ctx) -> Option<(f32, f32)> {
        let (go, dur, ph, _) = self.b[i];
        let age = t - go;
        if age >= dur {
            return None;
        }
        let k = ease_out_cubic(age / dur);
        let (_, hy) = self.home(i, c);
        let sway = (age * 3.0 + ph).sin() * 2.0 * (1.0 - k);
        Some((sway, (c.g.rows as f32 + 3.0 - hy) * (1.0 - k)))
    }
}

impl Effect for Bubbles {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (cols, rows) = (c.g.cols as f32, c.g.rows as f32);
        // Burbujitas de fondo que suben sin prisa.
        let fade = clamp01(t / 0.5) * (1.0 - smooth((t - 7.5) / 2.0));
        if fade > 0.0 {
            let n = ((c.g.cols * c.g.rows) / 120).clamp(4, 200) as u32;
            for k in 0..n {
                let p = (t * (0.12 + 0.12 * hf(k, self.seed, 1)) + hf(k, self.seed, 2)).fract();
                let y = rows + 1.0 - p * (rows + 2.0);
                let x = hf(k, self.seed, 3) * cols + (t * 1.5 + k as f32).sin() * 1.2;
                let ch = if hf(k, self.seed, 4) > 0.7 { 'o' } else { '°' };
                let col = scale(gradient(&c.pal, 0.4 + 0.4 * hf(k, self.seed, 5)), 0.35 * fade);
                c.g.put(x.round() as i32, y.round() as i32, ch, col);
            }
        }
        let (rx, ry) = (BUB_W as f32 * 0.75 + 1.0, BUB_H as f32 * 0.75 + 0.6);
        for i in 0..self.b.len() {
            let (go, dur, _, has) = self.b[i];
            let age = t - go;
            if !has || age < 0.0 || age >= dur + POP {
                continue;
            }
            let (hx, hy) = self.home(i, c);
            let rim = gradient(&c.pal, 0.3 + 0.5 * (hx - c.x0 as f32) / c.art.w.max(1) as f32);
            let (cx, cy, grow, f) = match self.offset(i, t, c) {
                Some((dx, dy)) => (hx + dx, hy + dy, 1.0, 1.0),
                // Al reventar, el borde se abre y se desvanece.
                None => {
                    let p = (age - dur) / POP;
                    (hx, hy, 1.0 + p * 0.8, 1.0 - p)
                }
            };
            for j in 0..RIM {
                let a = j as f32 / RIM as f32 * TAU;
                let (x, y) = (cx + a.cos() * rx * grow, cy + a.sin() * ry * grow);
                // Un brillo arriba a la izquierda.
                let lit = (a - 5.0 * FRAC_PI_4).abs() < 0.4;
                let (ch, col) = match (f < 1.0, lit) {
                    (true, _) => ('*', scale(mix(rim, WHITE, 0.5), f)),
                    (false, true) => ('°', mix(rim, WHITE, 0.7)),
                    (false, false) => ('o', scale(rim, 0.5)),
                };
                c.g.put(x.round() as i32, y.round() as i32, ch, col);
            }
        }
        for &(x, y, ch) in &c.art.cells {
            let i = ((y / BUB_H) * self.nbx + x / BUB_W) as usize;
            let (go, dur, _, _) = self.b[i];
            if t < go {
                continue;
            }
            let base = settle(&c.pal, &c.art, x, y, t);
            match self.offset(i, t, c) {
                // Dentro de la burbuja, más apagada.
                Some((dx, dy)) => c.g.put((c.x0 as f32 + x as f32 + dx).round() as i32, (c.y0 as f32 + y as f32 + dy).round() as i32, ch, scale(base, 0.55)),
                None => {
                    let col = shine(flash(base, t - go - dur, 0.5), &c.art, x, y, t, 9.0, 10.8);
                    c.g.put(c.x0 + x, c.y0 + y, ch, col);
                }
            }
        }
    }
}

// ── Radar ───────────────────────────────────────────────────────────────────

/// Giro del haz en radianes por segundo (una vuelta cada 2,4 s).
const SWEEP: f32 = 2.6;
const RADAR_ON: f32 = 0.6;
/// Ancho de la estela del haz, en radianes.
const TRAIL: f32 = 0.9;
const RING_EVERY: f32 = 9.0;
const BLIPS: u32 = 7;

/// Un haz de radar gira sobre el rótulo: cada letra aparece como un eco cuando el haz pasa por ella.
struct Radar {
    seed: u32,
    dir: f32,
}

pub fn radar(c: &mut Ctx) -> Box<dyn Effect> {
    let dir = if c.rng.chance(0.5) { 1.0 } else { -1.0 };
    Box::new(Radar { seed: c.rng.next_u64() as u32, dir })
}

impl Radar {
    /// Radianes desde que el haz pasó por el ángulo `a` por última vez.
    fn behind(&self, a: f32, t: f32) -> f32 {
        ((self.dir * SWEEP * (t - RADAR_ON) - a) * self.dir).rem_euclid(TAU)
    }
}

impl Effect for Radar {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (cx, cy) = art_center(c);
        let fade = clamp01(t / 0.6) * (1.0 - smooth((t - 8.0) / 2.0));
        let far = dist(c.g.cols as f32, c.g.rows as f32).max(1.0);
        if fade > 0.0 {
            for gy in 0..c.g.rows {
                for gx in 0..c.g.cols {
                    let (dx, dy) = (gx as f32 + 0.5 - cx, gy as f32 + 0.5 - cy);
                    let d = dist(dx, dy);
                    let trail = if t >= RADAR_ON { 1.0 - self.behind((dy * 2.0).atan2(dx), t) / TRAIL } else { 0.0 };
                    let col = gradient(&c.pal, 0.3 + 0.4 * d / far);
                    if trail > 0.0 {
                        let ch = if trail > 0.93 { '█' } else if trail > 0.6 { '▒' } else { '░' };
                        let col = if trail > 0.93 { mix(col, WHITE, 0.5) } else { col };
                        c.g.put(gx, gy, ch, scale(col, (0.06 + 0.45 * trail * trail * trail) * (1.0 - 0.5 * d / far) * fade));
                    } else if (d % RING_EVERY) < 0.7 || dx.abs() < 0.5 || dy.abs() < 0.5 {
                        // Anillos de distancia y la cruz del centro.
                        c.g.put(gx, gy, '·', scale(col, 0.35 * fade));
                    }
                }
            }
            // Contactos sueltos que se encienden al paso del haz y se apagan durante la vuelta.
            for k in 0..BLIPS {
                let a = hf(k, self.seed, 1) * TAU;
                let r = far * (0.25 + 0.45 * hf(k, self.seed, 2));
                let since = self.behind(a, t) / TAU;
                if t < RADAR_ON + (self.dir * a).rem_euclid(TAU) / SWEEP {
                    continue;
                }
                let col = scale(mix(gradient(&c.pal, 0.8), WHITE, 0.3), (1.0 - since) * fade);
                c.g.put((cx + a.cos() * r).round() as i32, (cy + a.sin() * r * 0.5).round() as i32, '+', col);
            }
        }
        for &(x, y, ch) in &c.art.cells {
            let (tx, ty) = ((c.x0 + x) as f32, (c.y0 + y) as f32);
            let a = ((ty + 0.5 - cy) * 2.0).atan2(tx + 0.5 - cx);
            let hit = RADAR_ON + (self.dir * a).rem_euclid(TAU) / SWEEP;
            if t < hit {
                continue;
            }
            // Mientras gira el radar, el eco se apaga un poco hasta la siguiente pasada.
            let since = self.behind(a, t) / TAU;
            let base = settle(&c.pal, &c.art, x, y, t);
            let echo = scale(base, 1.0 - 0.5 * since * fade);
            let col = shine(flash(echo, t - hit, 0.4), &c.art, x, y, t, 9.8, 11.4);
            c.g.put(tx as i32, ty as i32, ch, col);
        }
    }
}
