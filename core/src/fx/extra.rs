//! Segunda tanda de efectos: partículas, física sencilla y revelados radiales.

use super::{clamp01, ease_in_out_cubic, ease_out_cubic, flash, settle, shine, Ctx, Effect};
use crate::color::{gradient, mix, scale, Rgb, WHITE};
use crate::rng::{hash3, hf};
use core::f32::consts::{PI, TAU};

const ORANGE: Rgb = Rgb(255, 130, 40);
const YELLOW: Rgb = Rgb(255, 230, 130);

// ── Fuegos artificiales ─────────────────────────────────────────────────────

struct Rocket {
    x: f32,
    y: f32,
    target: f32,
    speed: f32,
    hue: f32,
}

struct Spark {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    hue: f32,
}

struct Fireworks {
    rockets: Vec<Rocket>,
    sparks: Vec<Spark>,
    /// Momento en que cada celda del rótulo queda encendida.
    shown: Vec<f32>,
    next: f32,
}

const FW_LAST: f32 = 6.5;
const FW_FORCE: f32 = 8.0;
const FW_SPARKS: usize = 36;
const FW_MAX_SPARKS: usize = 900;

pub fn fireworks(c: &mut Ctx) -> Box<dyn Effect> {
    Box::new(Fireworks { rockets: Vec::new(), sparks: Vec::new(), shown: vec![f32::INFINITY; c.art.cells.len()], next: 0.05 })
}

impl Effect for Fireworks {
    fn draw(&mut self, t: f32, dt: f32, c: &mut Ctx) {
        let n = c.art.cells.len();
        // Cada cohete apunta a una celda que aún esté apagada.
        while n > 0 && t >= self.next && self.next < FW_LAST {
            self.next += c.rng.range(0.28, 0.5);
            let mut i = c.rng.below(n);
            for _ in 0..8 {
                if self.shown[i].is_infinite() {
                    break;
                }
                i = c.rng.below(n);
            }
            let (x, y, _) = c.art.cells[i];
            self.rockets.push(Rocket { x: (c.x0 + x) as f32, y: c.g.rows as f32, target: (c.y0 + y) as f32, speed: c.rng.range(38.0, 56.0), hue: c.rng.f32() });
        }
        let radius = (c.art.w as f32 / 6.0).max(7.0);
        let mut i = 0;
        while i < self.rockets.len() {
            self.rockets[i].y -= self.rockets[i].speed * dt;
            if self.rockets[i].y > self.rockets[i].target {
                i += 1;
                continue;
            }
            let r = self.rockets.swap_remove(i);
            for (k, &(x, y, _)) in c.art.cells.iter().enumerate() {
                let d = ((c.x0 + x) as f32 - r.x).hypot(((c.y0 + y) as f32 - r.target) * 2.0);
                if d < radius && self.shown[k].is_infinite() {
                    self.shown[k] = t + d / radius * 0.3;
                }
            }
            for _ in 0..FW_SPARKS {
                if self.sparks.len() >= FW_MAX_SPARKS {
                    break;
                }
                let (a, v) = (c.rng.range(0.0, TAU), c.rng.range(5.0, 22.0));
                self.sparks.push(Spark { x: r.x, y: r.target, vx: a.cos() * v, vy: a.sin() * v * 0.5, life: c.rng.range(0.5, 1.2), hue: r.hue + c.rng.range(-0.08, 0.08) });
            }
        }
        if t >= FW_FORCE {
            for (k, s) in self.shown.iter_mut().enumerate().filter(|(_, s)| s.is_infinite()) {
                *s = FW_FORCE + hf(k as u32, 3, 9) * 0.6;
            }
        }
        self.sparks.retain_mut(|s| {
            s.x += s.vx * dt;
            s.y += s.vy * dt;
            s.vy += 6.0 * dt;
            s.vx *= (1.0 - 1.5 * dt).max(0.0);
            s.life -= dt;
            s.life > 0.0
        });
        for (k, &(x, y, ch)) in c.art.cells.iter().enumerate() {
            let s = self.shown[k];
            if t >= s {
                let col = shine(flash(settle(&c.pal, &c.art, x, y, t), t - s, 0.7), &c.art, x, y, t, 10.0, 12.0);
                c.g.put(c.x0 + x, c.y0 + y, ch, col);
            }
        }
        for r in &self.rockets {
            let (x, y) = (r.x as i32, r.y as i32);
            c.g.put(x, y, '•', WHITE);
            c.g.put(x, y + 1, '·', scale(YELLOW, 0.7));
            c.g.put(x, y + 2, '·', scale(ORANGE, 0.4));
        }
        for s in &self.sparks {
            let k = clamp01(s.life / 0.8);
            let ch = if s.life > 0.75 {
                '✶'
            } else if s.life > 0.35 {
                '•'
            } else {
                '·'
            };
            c.g.put(s.x.round() as i32, s.y.round() as i32, ch, scale(mix(gradient(&c.pal, s.hue), WHITE, k * 0.5), 0.35 + 0.65 * k));
        }
    }
}

// ── Remolino ────────────────────────────────────────────────────────────────

struct Vortex {
    /// Radio y ángulo de la órbita inicial, y cuándo empieza a caer hacia su sitio.
    p: Vec<(f32, f32, f32)>,
}

const VORTEX_DUR: f32 = 3.6;

pub fn vortex(c: &mut Ctx) -> Box<dyn Effect> {
    let max_r = c.g.cols.max(c.g.rows * 2) as f32 * 0.55;
    let p = (0..c.art.cells.len()).map(|_| (c.rng.range(0.2, 1.0) * max_r, c.rng.range(0.0, TAU), c.rng.range(0.3, 2.3))).collect();
    Box::new(Vortex { p })
}

impl Effect for Vortex {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (cx, cy) = (c.g.cols as f32 / 2.0, c.g.rows as f32 / 2.0);
        let fade_in = clamp01(t / 0.7);
        for (i, &(x, y, ch)) in c.art.cells.iter().enumerate() {
            let (r, a0, start) = self.p[i];
            let p = ease_in_out_cubic((t - start) / VORTEX_DUR);
            // Las de dentro giran más rápido, como en un remolino de verdad.
            let a = a0 + t * (1.0 + 40.0 / (r + 20.0));
            let (ox, oy) = (cx + r * a.cos(), cy + r * a.sin() * 0.5);
            let (tx, ty) = ((c.x0 + x) as f32, (c.y0 + y) as f32);
            let base = settle(&c.pal, &c.art, x, y, t);
            let arrive = start + VORTEX_DUR;
            let (glyph, col) = if t >= arrive {
                (ch, shine(flash(base, t - arrive, 0.5), &c.art, x, y, t, 7.0, 8.8))
            } else {
                let glyph = if p < 0.55 { '•' } else { ch };
                (glyph, scale(mix(base, WHITE, 0.25 * (1.0 - p)), (0.6 + 0.4 * p) * fade_in))
            };
            c.g.put((ox + (tx - ox) * p).round() as i32, (oy + (ty - oy) * p).round() as i32, glyph, col);
        }
    }
}

// ── Rebote ──────────────────────────────────────────────────────────────────

struct Bounce {
    /// Cuándo empieza a caer cada celda (todas las de una misma letra, a la vez).
    start: Vec<f32>,
}

const BOUNCE_DUR: f32 = 1.5;

fn bounce_out(p: f32) -> f32 {
    let p = clamp01(p);
    let (n, d) = (7.5625, 2.75);
    if p < 1.0 / d {
        n * p * p
    } else if p < 2.0 / d {
        let p = p - 1.5 / d;
        n * p * p + 0.75
    } else if p < 2.5 / d {
        let p = p - 2.25 / d;
        n * p * p + 0.9375
    } else {
        let p = p - 2.625 / d;
        n * p * p + 0.984375
    }
}

pub fn bounce(c: &mut Ctx) -> Box<dyn Effect> {
    let (w, h) = (c.art.w.max(1) as usize, c.art.h.max(1) as usize);
    // Bandas: grupos de filas separados por filas vacías (título y subtítulo).
    let mut row_used = vec![false; h];
    for &(_, y, _) in &c.art.cells {
        row_used[y as usize] = true;
    }
    let mut band_of = vec![0usize; h];
    let mut bands = 0;
    for y in 0..h {
        if row_used[y] && (y == 0 || !row_used[y - 1]) {
            bands += 1;
        }
        band_of[y] = bands.max(1) - 1;
    }
    let bands = bands.max(1);
    // Dentro de cada banda, cada tramo de columnas seguidas es una letra.
    let mut col_used = vec![false; bands * w];
    for &(x, y, _) in &c.art.cells {
        col_used[band_of[y as usize] * w + x as usize] = true;
    }
    let mut letter_start = vec![0.0f32; bands * w];
    for b in 0..bands {
        let cols = &col_used[b * w..(b + 1) * w];
        let mut run = vec![0usize; w];
        let mut runs = 0;
        for x in 0..w {
            if cols[x] && (x == 0 || !cols[x - 1]) {
                runs += 1;
            }
            run[x] = runs.max(1) - 1;
        }
        // Arte sin huecos entre letras: se trocea en tiras de seis columnas.
        if runs < 3 && w > 12 {
            runs = w.div_ceil(6);
            run.iter_mut().enumerate().for_each(|(x, r)| *r = x / 6);
        }
        let step = (2.2 / runs.max(1) as f32).min(0.2);
        for x in 0..w {
            letter_start[b * w + x] = 0.05 + b as f32 * 0.9 + run[x] as f32 * step;
        }
    }
    let start = c.art.cells.iter().map(|&(x, y, _)| letter_start[band_of[y as usize] * w + x as usize]).collect();
    Box::new(Bounce { start })
}

impl Effect for Bounce {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let drop = (c.y0 + c.art.h + 2) as f32;
        for (i, &(x, y, ch)) in c.art.cells.iter().enumerate() {
            let start = self.start[i];
            if t < start {
                continue;
            }
            let off = -drop * (1.0 - bounce_out((t - start) / BOUNCE_DUR));
            let base = settle(&c.pal, &c.art, x, y, t);
            let hit = start + BOUNCE_DUR / 2.75;
            let col = if t < hit { mix(base, WHITE, 0.2) } else { flash(base, t - hit, 0.5) };
            c.g.put(c.x0 + x, c.y0 + y + off.round() as i32, ch, shine(col, &c.art, x, y, t, 6.2, 8.0));
        }
    }
}

// ── Láser ───────────────────────────────────────────────────────────────────

struct Laser {
    /// x, y, vx, vy, vida
    sparks: Vec<[f32; 5]>,
    salt: u32,
}

const LASER_T0: f32 = 0.1;
const LASER_DUR: f32 = 5.2;

pub fn laser(c: &mut Ctx) -> Box<dyn Effect> {
    Box::new(Laser { sparks: Vec::new(), salt: c.rng.next_u64() as u32 })
}

impl Effect for Laser {
    fn draw(&mut self, t: f32, dt: f32, c: &mut Ctx) {
        let (w, h) = (c.art.w.max(1), c.art.h.max(1));
        let span = (w + 1) as f32;
        let q = (t - LASER_T0) / LASER_DUR;
        let head = q * span - 0.5;
        for &(x, y, ch) in &c.art.cells {
            let passed = LASER_T0 + (x as f32 + 0.5) / span * LASER_DUR;
            if t < passed {
                continue;
            }
            let since = t - passed;
            let hot = mix(WHITE, ORANGE, clamp01(since / 0.2));
            let col = mix(hot, settle(&c.pal, &c.art, x, y, t), ease_out_cubic(since / 1.1));
            c.g.put(c.x0 + x, c.y0 + y, ch, shine(col, &c.art, x, y, t, 7.0, 8.8));
        }
        let firing = (0.0..1.0).contains(&q);
        if firing {
            // El rayo sale del centro de arriba y tiembla por la columna que está grabando.
            let step = (t * 30.0) as u32;
            let (sx, sy) = (c.g.cols as f32 / 2.0, 0.0f32);
            let (tx, ty) = (c.x0 as f32 + head, (c.y0 + (hash3(step, 1, self.salt) % h as u32) as i32) as f32);
            let (dx, dy) = (tx - sx, ty - sy);
            let glyph = if dx.abs() < dy.abs() * 0.8 {
                '|'
            } else if dx.abs() > dy.abs() * 5.0 {
                '-'
            } else if dx * dy < 0.0 {
                '/'
            } else {
                '\\'
            };
            let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as i32;
            let beam = mix(gradient(&c.pal, 0.5), WHITE, 0.7);
            for k in 0..steps {
                let f = k as f32 / steps as f32;
                let flicker = 0.55 + 0.45 * hf(k as u32, step, self.salt);
                c.g.put((sx + dx * f).round() as i32, (sy + dy * f).round() as i32, glyph, scale(beam, (0.7 + 0.3 * flicker) * (0.75 + 0.25 * f)));
            }
            c.g.put(tx.round() as i32, ty.round() as i32, '✶', WHITE);
            for _ in 0..((dt * 70.0).ceil() as usize).min(8) {
                self.sparks.push([tx, ty, c.rng.range(-11.0, 11.0), c.rng.range(-9.0, -1.0), c.rng.range(0.25, 0.7)]);
            }
        }
        self.sparks.retain_mut(|s| {
            s[0] += s[2] * dt;
            s[1] += s[3] * dt;
            s[3] += 24.0 * dt;
            s[4] -= dt;
            s[4] > 0.0
        });
        for s in &self.sparks {
            let k = clamp01(s[4] / 0.5);
            c.g.put(s[0].round() as i32, s[1].round() as i32, if k > 0.5 { '•' } else { '·' }, scale(mix(ORANGE, YELLOW, k), 0.4 + 0.6 * k));
        }
    }
}

// ── Cristal ─────────────────────────────────────────────────────────────────

struct Crystal {
    /// Cuándo se congela cada celda: crece desde unas pocas semillas.
    freeze: Vec<f32>,
    salt: u32,
}

const FROST: [char; 3] = ['░', '▒', '▓'];
const FROST_STEP: f32 = 0.12;

pub fn crystal(c: &mut Ctx) -> Box<dyn Effect> {
    let n = c.art.cells.len();
    let seeds: Vec<(f32, f32)> = (0..4.min(n)).map(|_| c.art.cells[c.rng.below(n)]).map(|(x, y, _)| (x as f32, y as f32)).collect();
    let dist: Vec<f32> = c.art.cells.iter().map(|&(x, y, _)| seeds.iter().map(|&(sx, sy)| (x as f32 - sx).hypot((y as f32 - sy) * 2.0)).fold(f32::INFINITY, f32::min)).collect();
    let far = dist.iter().copied().fold(1.0f32, f32::max);
    let freeze = dist.iter().map(|d| 0.05 + d / far * 4.2 + c.rng.range(0.0, 0.3)).collect();
    Box::new(Crystal { freeze, salt: c.rng.next_u64() as u32 })
}

impl Effect for Crystal {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let tick = (t * 7.0) as u32;
        for (i, &(x, y, ch)) in c.art.cells.iter().enumerate() {
            let since = t - self.freeze[i];
            if since < 0.0 {
                continue;
            }
            let base = settle(&c.pal, &c.art, x, y, t);
            let (gx, gy) = (c.x0 + x, c.y0 + y);
            let stage = (since / FROST_STEP) as usize;
            if stage < FROST.len() {
                c.g.put(gx, gy, FROST[stage], mix(base, WHITE, 0.65));
                continue;
            }
            let mut col = flash(base, since - FROST_STEP * FROST.len() as f32, 0.6);
            // Destellos sueltos sobre el hielo ya formado.
            if hash3(i as u32, tick, self.salt) % 90 < 1 {
                col = mix(col, WHITE, 0.75);
            }
            c.g.put(gx, gy, ch, shine(col, &c.art, x, y, t, 6.5, 8.3));
        }
    }
}

// ── Onda ────────────────────────────────────────────────────────────────────

struct Ripple;

const RIPPLE_T0: f32 = 0.05;
const RIPPLE_EVERY: f32 = 1.5;
const RIPPLE_RINGS: usize = 4;
const RIPPLE_CROSS: f32 = 1.6;
const RIPPLE_WIDTH: f32 = 6.0;

pub fn ripple(_c: &mut Ctx) -> Box<dyn Effect> {
    Box::new(Ripple)
}

impl Effect for Ripple {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (w, h) = (c.art.w as f32, c.art.h as f32);
        let (cx, cy) = (w / 2.0, h / 2.0);
        let far = (w / 2.0).hypot(h).max(1.0);
        let speed = far / RIPPLE_CROSS;
        let radius = |k: usize| (t - RIPPLE_T0 - k as f32 * RIPPLE_EVERY) * speed;
        // Los anillos siguen más allá del rótulo, cada vez más tenues.
        let (sx, sy) = (c.x0 as f32 + cx, c.y0 as f32 + cy);
        for k in 0..RIPPLE_RINGS {
            let r = radius(k);
            let fade = clamp01(1.0 - r / (far * 2.2)) * 0.6_f32.powi(k as i32);
            if r <= 0.0 || fade <= 0.02 {
                continue;
            }
            let dots = ((r * TAU / 1.6) as i32).clamp(12, 720);
            for j in 0..dots {
                let a = j as f32 / dots as f32 * TAU;
                c.g.put((sx + r * a.cos()).round() as i32, (sy + r * a.sin() * 0.5).round() as i32, '·', scale(mix(gradient(&c.pal, a / TAU), WHITE, 0.3), fade));
            }
        }
        for &(x, y, ch) in &c.art.cells {
            let d = (x as f32 - cx).hypot((y as f32 - cy) * 2.0);
            if d > radius(0) {
                continue;
            }
            // Cada anillo levanta las celdas al pasar y las deja brillando un instante.
            let (mut lift, mut glow) = (0.0f32, 0.0f32);
            for k in 0..RIPPLE_RINGS {
                let delta = (d - radius(k)).abs();
                if delta < RIPPLE_WIDTH {
                    let bump = (delta / RIPPLE_WIDTH * PI / 2.0).cos() * 0.6_f32.powi(k as i32);
                    lift += bump * 2.2;
                    glow = glow.max(bump);
                }
            }
            let col = mix(settle(&c.pal, &c.art, x, y, t), WHITE, glow * 0.75);
            c.g.put(c.x0 + x, c.y0 + y - lift.round() as i32, ch, shine(col, &c.art, x, y, t, 7.4, 9.2));
        }
    }
}
