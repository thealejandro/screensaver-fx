//! Efectos nuevos al estilo de terminaltexteffects: el rótulo se arma con distintas animaciones.

use super::{clamp01, ease_in_out_cubic, ease_out_cubic, flash, noise_ch, settle, shine, Ctx, Effect};
use crate::color::{gradient, mix, scale, Rgb, WHITE};
use core::f32::consts::TAU;

fn wf(c: &Ctx) -> f32 {
    c.art.w.max(1) as f32
}

fn hf(c: &Ctx) -> f32 {
    c.art.h.max(1) as f32
}

// ── Descifrado ──────────────────────────────────────────────────────────────

struct Decrypt {
    p: Vec<(f32, f32, f32)>,
    salt: u32,
}

pub fn decrypt(c: &mut Ctx) -> Box<dyn Effect> {
    let p = (0..c.art.cells.len()).map(|_| (c.rng.range(0.0, 1.0), c.rng.range(1.6, 4.8), c.rng.range(8.0, 20.0))).collect();
    Box::new(Decrypt { p, salt: c.rng.next_u64() as u32 })
}

impl Effect for Decrypt {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        for (i, &(x, y, ch)) in c.art.cells.iter().enumerate() {
            let (appear, lock, rate) = self.p[i];
            if t < appear {
                continue;
            }
            let base = settle(&c.pal, &c.art, x, y, t);
            if t < lock {
                let n = noise_ch(i as u32, (t * rate) as u32, self.salt);
                c.g.put(c.x0 + x, c.y0 + y, n, scale(base, 0.45));
            } else {
                let col = shine(flash(base, t - lock, 0.5), &c.art, x, y, t, 6.5, 8.3);
                c.g.put(c.x0 + x, c.y0 + y, ch, col);
            }
        }
    }
}

// ── Lluvia ──────────────────────────────────────────────────────────────────

struct Rain {
    p: Vec<(f32, f32, f32)>,
}

pub fn rain(c: &mut Ctx) -> Box<dyn Effect> {
    let p = (0..c.art.cells.len()).map(|_| (c.rng.range(0.0, 3.0), c.rng.range(14.0, 34.0), c.rng.range(0.0, 3.0))).collect();
    Box::new(Rain { p })
}

impl Effect for Rain {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        for (i, &(x, y, ch)) in c.art.cells.iter().enumerate() {
            let (start, speed, off) = self.p[i];
            if t < start {
                continue;
            }
            let target = (c.y0 + y) as f32;
            let pos = -off + (t - start) * speed;
            let base = settle(&c.pal, &c.art, x, y, t);
            if pos < target {
                c.g.put(c.x0 + x, pos as i32, ch, mix(base, WHITE, 0.25));
            } else {
                let land = start + (target + off) / speed;
                c.g.put(c.x0 + x, c.y0 + y, ch, shine(flash(base, t - land, 0.45), &c.art, x, y, t, 7.0, 8.8));
            }
        }
    }
}

// ── Rayos ───────────────────────────────────────────────────────────────────

const TRAIL: [char; 4] = ['█', '▓', '▒', '░'];
const BEAM_SPEED: f32 = 70.0;
const TRAIL_LEN: i32 = 10;

struct Beams {
    rows: Vec<f32>,
}

pub fn beams(c: &mut Ctx) -> Box<dyn Effect> {
    let rows = (0..c.art.h).map(|r| r as f32 * 0.09 + c.rng.range(0.0, 0.6)).collect();
    Box::new(Beams { rows })
}

impl Effect for Beams {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let w = c.art.w;
        let head = |r: i32, sr: f32| {
            let hp = (t - sr) * BEAM_SPEED;
            if r % 2 == 0 {
                -2.0 + hp
            } else {
                (w + 1) as f32 - hp
            }
        };
        for &(x, y, ch) in &c.art.cells {
            let sr = self.rows[y as usize];
            let passed = if y % 2 == 0 { sr + (x + 2) as f32 / BEAM_SPEED } else { sr + (w + 1 - x) as f32 / BEAM_SPEED };
            if t >= passed {
                let col = shine(flash(settle(&c.pal, &c.art, x, y, t), t - passed, 0.6), &c.art, x, y, t, 6.0, 8.0);
                c.g.put(c.x0 + x, c.y0 + y, ch, col);
            }
        }
        for r in 0..c.art.h {
            let sr = self.rows[r as usize];
            if t < sr {
                continue;
            }
            let hx = head(r, sr) as i32;
            let dir = if r % 2 == 0 { -1 } else { 1 };
            for k in 0..TRAIL_LEN {
                let x = hx + dir * k;
                if x < -2 || x > w + 1 {
                    continue;
                }
                let fade = 1.0 - k as f32 / TRAIL_LEN as f32;
                let col = mix(gradient(&c.pal, x as f32 / w.max(1) as f32), WHITE, if k == 0 { 0.8 } else { 0.2 });
                c.g.put(c.x0 + x, c.y0 + r, TRAIL[(k * 4 / TRAIL_LEN) as usize], scale(col, fade));
            }
        }
    }
}

// ── Deslizar ────────────────────────────────────────────────────────────────

struct Slide;

pub fn slide(_c: &mut Ctx) -> Box<dyn Effect> {
    Box::new(Slide)
}

const SLIDE_DUR: f32 = 1.4;

impl Effect for Slide {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let from_left = -(c.x0 + c.art.w + 2) as f32;
        let from_right = (c.g.cols - c.x0 + 2) as f32;
        for &(x, y, ch) in &c.art.cells {
            let start = y as f32 * 0.07;
            if t < start {
                continue;
            }
            let p = (t - start) / SLIDE_DUR;
            let off = if y % 2 == 0 { from_left } else { from_right } * (1.0 - ease_out_cubic(p));
            let base = settle(&c.pal, &c.art, x, y, t);
            let col = if p < 1.0 { base } else { mix(flash(base, t - start - SLIDE_DUR, 0.6), base, 0.4) };
            c.g.put(c.x0 + x + off.round() as i32, c.y0 + y, ch, shine(col, &c.art, x, y, t, 5.5, 7.3));
        }
    }
}

// ── Dispersión ──────────────────────────────────────────────────────────────

struct Scatter {
    p: Vec<(f32, f32, f32)>,
}

pub fn scatter(c: &mut Ctx) -> Box<dyn Effect> {
    let (cols, rows) = (c.g.cols as f32, c.g.rows as f32);
    let p = (0..c.art.cells.len()).map(|_| (c.rng.range(0.0, cols), c.rng.range(0.0, rows), 0.9 + c.rng.range(0.0, 0.9))).collect();
    Box::new(Scatter { p })
}

const SCATTER_DUR: f32 = 2.2;

impl Effect for Scatter {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let fade_in = clamp01(t / 0.6);
        for (i, &(x, y, ch)) in c.art.cells.iter().enumerate() {
            let (sx, sy, start) = self.p[i];
            let (tx, ty) = ((c.x0 + x) as f32, (c.y0 + y) as f32);
            let p = ease_in_out_cubic((t - start) / SCATTER_DUR);
            let base = settle(&c.pal, &c.art, x, y, t);
            let arrive = start + SCATTER_DUR;
            let col = if t < arrive { scale(base, (0.45 + 0.55 * p) * fade_in) } else { shine(flash(base, t - arrive, 0.5), &c.art, x, y, t, 6.0, 7.8) };
            c.g.put((sx + (tx - sx) * p).round() as i32, (sy + (ty - sy) * p).round() as i32, ch, col);
        }
    }
}

// ── Fuego ───────────────────────────────────────────────────────────────────

const FIRE: [char; 3] = ['▓', '▒', '░'];
const ASH: Rgb = Rgb(70, 70, 78);
const ORANGE: Rgb = Rgb(255, 120, 30);
const YELLOW: Rgb = Rgb(255, 225, 110);
const RED: Rgb = Rgb(190, 35, 10);
const BURN: f32 = 0.7;
const COOL: f32 = 0.8;

struct Burn {
    ign: Vec<f32>,
    /// x, y (en pantalla), velocidad, vida restante
    embers: Vec<(f32, f32, f32, f32)>,
}

pub fn burn(c: &mut Ctx) -> Box<dyn Effect> {
    let h = hf(c);
    let ign = c.art.cells.iter().map(|&(_, y, _)| 0.6 + (h - 1.0 - y as f32) / h * 3.2).collect::<Vec<_>>();
    let ign = ign.into_iter().map(|v| v + c.rng.range(0.0, 0.6)).collect();
    Box::new(Burn { ign, embers: Vec::new() })
}

impl Effect for Burn {
    fn draw(&mut self, t: f32, dt: f32, c: &mut Ctx) {
        // Brasas que suben desde las celdas encendidas.
        let n = c.art.cells.len();
        if n > 0 && t < 5.5 {
            let spawn = (dt * 60.0).ceil() as usize;
            for _ in 0..spawn {
                let i = c.rng.below(n);
                let since = t - self.ign[i];
                if (0.0..BURN).contains(&since) {
                    let (x, y, _) = c.art.cells[i];
                    self.embers.push(((c.x0 + x) as f32 + c.rng.range(-0.5, 0.5), (c.y0 + y) as f32, c.rng.range(4.0, 10.0), c.rng.range(0.4, 1.1)));
                }
            }
        }
        self.embers.retain_mut(|e| {
            e.1 -= e.2 * dt;
            e.0 += (e.1 * 0.7 + t * 3.0).sin() * dt * 1.5;
            e.3 -= dt;
            e.3 > 0.0 && e.1 >= 0.0
        });
        for e in &self.embers {
            let k = clamp01(e.3 / 0.8);
            c.g.put(e.0 as i32, e.1 as i32, if e.3 > 0.5 { '•' } else { '·' }, scale(mix(RED, YELLOW, k), 0.4 + 0.6 * k));
        }
        for (i, &(x, y, ch)) in c.art.cells.iter().enumerate() {
            let since = t - self.ign[i];
            let (gx, gy) = (c.x0 + x, c.y0 + y);
            if since < 0.0 {
                c.g.put(gx, gy, ch, ASH);
            } else if since < BURN {
                let p = since / BURN;
                c.g.put(gx, gy, FIRE[((p * 3.0) as usize).min(2)], mix(YELLOW, RED, p));
            } else {
                let base = settle(&c.pal, &c.art, x, y, t);
                let col = mix(ORANGE, base, ease_out_cubic((since - BURN) / COOL));
                c.g.put(gx, gy, ch, shine(col, &c.art, x, y, t, 7.5, 9.5));
            }
        }
    }
}

// ── Matrix ──────────────────────────────────────────────────────────────────

struct Drop {
    x: i32,
    y: f32,
    speed: f32,
    len: i32,
}

struct Matrix {
    drops: Vec<Drop>,
    /// Momento en que se reveló cada celda del rótulo (índice x + y * w).
    shown: Vec<f32>,
    lookup: Vec<i32>,
    salt: u32,
}

const MATRIX_REVEAL: f32 = 1.2;
const MATRIX_FORCE: f32 = 8.0;
const MATRIX_STOP: f32 = 8.5;

fn new_drop(c: &mut Ctx, x: i32, top: f32) -> Drop {
    Drop { x, y: top, speed: c.rng.range(10.0, 30.0), len: 6 + c.rng.below(13) as i32 }
}

pub fn matrix(c: &mut Ctx) -> Box<dyn Effect> {
    let rows = c.g.rows as f32;
    let drops = (0..c.g.cols)
        .map(|x| {
            let top = -c.rng.range(0.0, rows);
            new_drop(c, x, top)
        })
        .collect();
    let (w, h) = (c.art.w.max(1), c.art.h.max(1));
    let mut lookup = vec![-1; (w * h) as usize];
    for (i, &(x, y, _)) in c.art.cells.iter().enumerate() {
        lookup[(x + y * w) as usize] = i as i32;
    }
    Box::new(Matrix { drops, shown: vec![f32::INFINITY; c.art.cells.len()], lookup, salt: c.rng.next_u64() as u32 })
}

impl Effect for Matrix {
    fn draw(&mut self, t: f32, dt: f32, c: &mut Ctx) {
        let (w, h, rows) = (c.art.w, c.art.h, c.g.rows);
        let mut i = 0;
        while i < self.drops.len() {
            let prev = self.drops[i].y as i32;
            self.drops[i].y += self.drops[i].speed * dt;
            let d = &self.drops[i];
            let ax = d.x - c.x0;
            if t > MATRIX_REVEAL && (0..w).contains(&ax) {
                for r in prev.max(c.y0)..=(d.y as i32).min(c.y0 + h - 1) {
                    let idx = self.lookup[(ax + (r - c.y0) * w) as usize];
                    if idx >= 0 && self.shown[idx as usize].is_infinite() {
                        self.shown[idx as usize] = t;
                    }
                }
            }
            if d.y as i32 - d.len > rows {
                if t < MATRIX_STOP {
                    let x = d.x;
                    let top = -c.rng.range(0.0, 6.0);
                    self.drops[i] = new_drop(c, x, top);
                } else {
                    self.drops.swap_remove(i);
                    continue;
                }
            }
            i += 1;
        }
        if t >= MATRIX_FORCE {
            for s in self.shown.iter_mut().filter(|s| s.is_infinite()) {
                *s = MATRIX_FORCE;
            }
        }
        let step = (t * 12.0) as u32;
        for d in &self.drops {
            let hy = d.y as i32;
            for k in 0..d.len {
                let y = hy - k;
                if y < 0 || y >= rows {
                    continue;
                }
                let ch = noise_ch(d.x as u32, y as u32, step.wrapping_add(k as u32 * 7) ^ self.salt);
                let p = k as f32 / d.len as f32;
                let col = if k == 0 { WHITE } else { scale(gradient(&c.pal, 0.15 + p * 0.6), (1.0 - p) * 0.85) };
                c.g.put(d.x, y, ch, col);
            }
        }
        for (i, &(x, y, ch)) in c.art.cells.iter().enumerate() {
            let s = self.shown[i];
            if t >= s {
                let col = shine(flash(settle(&c.pal, &c.art, x, y, t), t - s, 0.6), &c.art, x, y, t, 10.0, 12.0);
                c.g.put(c.x0 + x, c.y0 + y, ch, col);
            }
        }
    }
}

// ── Olas ────────────────────────────────────────────────────────────────────

const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

struct Waves;

pub fn waves(_c: &mut Ctx) -> Box<dyn Effect> {
    Box::new(Waves)
}

impl Effect for Waves {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (w, h) = (wf(c), hf(c));
        for &(x, y, ch) in &c.art.cells {
            let (xf, yf) = (x as f32, y as f32);
            let appear = 1.2 * xf / w;
            if t < appear {
                continue;
            }
            let done = 5.5 + 1.6 * xf / w + 0.3 * yf / h;
            let (gx, gy) = (c.x0 + x, c.y0 + y);
            if t < done {
                let v = (xf * 0.35 - t * 5.0 + yf * 0.6).sin() * 0.5 + 0.5;
                let bar = BARS[((v * 7.99) as usize).min(7)];
                c.g.put(gx, gy, bar, scale(gradient(&c.pal, v * 0.5 + xf / w * 0.5), 0.55 + 0.45 * v));
            } else {
                c.g.put(gx, gy, ch, flash(settle(&c.pal, &c.art, x, y, t), t - done, 0.5));
            }
        }
    }
}

// ── Focos ───────────────────────────────────────────────────────────────────

struct Spotlights {
    p: [(f32, f32, f32, f32); 3],
}

pub fn spotlights(c: &mut Ctx) -> Box<dyn Effect> {
    let mut p = [(0.0, 0.0, 0.0, 0.0); 3];
    for l in p.iter_mut() {
        *l = (c.rng.range(0.5, 1.1), c.rng.range(0.7, 1.4), c.rng.range(0.0, TAU), c.rng.range(0.0, TAU));
    }
    Box::new(Spotlights { p })
}

impl Effect for Spotlights {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (w, h) = (wf(c), hf(c));
        let k = ease_in_out_cubic((t - 6.0) / 2.5);
        let fade = clamp01(t / 0.8);
        let radius = 8.0 + k * w;
        let lights: Vec<(f32, f32)> = self
            .p
            .iter()
            .map(|&(a, b, pa, pb)| {
                let lx = w / 2.0 + w * 0.45 * (t * a + pa).sin();
                let ly = h / 2.0 + h * 0.6 * (t * b + pb).sin();
                (lx + (w / 2.0 - lx) * k, ly + (h / 2.0 - ly) * k)
            })
            .collect();
        for &(x, y, ch) in &c.art.cells {
            let (xf, yf) = (x as f32, y as f32);
            let light = lights.iter().map(|&(lx, ly)| clamp01(1.0 - (xf - lx).hypot((yf - ly) * 2.0) / radius)).fold(0.0f32, f32::max);
            let light = light * light * (3.0 - 2.0 * light);
            let col = scale(settle(&c.pal, &c.art, x, y, t), (0.10 + 0.9 * light) * fade);
            c.g.put(c.x0 + x, c.y0 + y, ch, shine(col, &c.art, x, y, t, 9.0, 10.8));
        }
    }
}

// ── Barrido ─────────────────────────────────────────────────────────────────

struct Wipe;

pub fn wipe(_c: &mut Ctx) -> Box<dyn Effect> {
    Box::new(Wipe)
}

impl Effect for Wipe {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let span = (c.art.w + c.art.h * 2 + 6) as f32;
        let front = (t - 0.3) / 2.8 * span - 3.0;
        for &(x, y, ch) in &c.art.cells {
            let d = (x + y * 2) as f32;
            if d > front + 1.0 {
                continue;
            }
            let col = if d > front - 1.0 { WHITE } else { mix(WHITE, settle(&c.pal, &c.art, x, y, t), clamp01((front - d) / 6.0)) };
            c.g.put(c.x0 + x, c.y0 + y, ch, shine(col, &c.art, x, y, t, 5.5, 7.3));
        }
    }
}
