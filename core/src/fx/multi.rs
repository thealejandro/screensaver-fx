//! Efectos de escritorio: usan todas las pantallas como un solo lienzo. Cada motor calcula el
//! efecto entero a partir del tiempo y de la semilla común (sin estado que dependa de su propia
//! rejilla) y dibuja solo lo que cae en su pantalla, así que lo que sale por el borde de un
//! monitor aparece en el de al lado. Con un solo monitor se ven igual de bien.

use super::{clamp01, ease_in_out_cubic, flash, settle, shine, Ctx, Effect};
use crate::color::{gradient, mix, scale, Rgb, WHITE};
use crate::desk::View;
use crate::rng::hf;
use core::f32::consts::TAU;

/// Dibuja en esta pantalla lo que cae en el punto (wx, wy) del escritorio.
fn put_world(c: &mut Ctx, v: &View, wx: f32, wy: f32, ch: char, col: Rgb) {
    let (x, y) = v.to_cell(wx, wy);
    c.g.put(x, y, ch, col);
}

/// Centro de la celda (x, y) del rótulo en la pantalla `k`, en el escritorio.
fn banner_cell(c: &Ctx, k: usize, x: i32, y: i32) -> (f32, f32) {
    let v = &c.desk.views[k];
    v.to_world((v.x0 + x) as f32, (v.y0 + y) as f32)
}

fn lerp2(a: (f32, f32), b: (f32, f32), p: f32) -> (f32, f32) {
    (a.0 + (b.0 - a.0) * p, a.1 + (b.1 - a.1) * p)
}

// ── Relevo ──────────────────────────────────────────────────────────────────
// El rótulo se arma en el monitor de la izquierda y salta, letra a letra y en arco, al siguiente
// hasta llegar al último; luego se enciende a la vez en todos. Con un monitor sale por arriba y
// vuelve a caer en su sitio.

const RELAY_IN: f32 = 1.4;
const RELAY_HOPS: f32 = 7.0;
const RELAY_MAX_HOP: f32 = 2.6;

struct Relay {
    route: Vec<usize>,
    hop: f32,
    jit: Vec<f32>,
}

pub fn relay(c: &mut Ctx) -> Box<dyn Effect> {
    let mut route = c.desk.order.clone();
    if route.len() == 1 {
        route.push(route[0]);
    }
    let hop = (RELAY_HOPS / (route.len() - 1) as f32).min(RELAY_MAX_HOP);
    let jit = (0..c.art.cells.len()).map(|_| c.rng.f32()).collect();
    Box::new(Relay { route, hop, jit })
}

impl Relay {
    /// Salida y llegada de la celda `i` en el salto `h`.
    fn times(&self, c: &Ctx, h: usize, i: usize, x: i32) -> (f32, f32) {
        let (a, b) = (c.desk.views[self.route[h]], c.desk.views[self.route[h + 1]]);
        let dx = b.banner_center(c.art.w, c.art.h).0 - a.banner_center(c.art.w, c.art.h).0;
        let xf = x as f32 / c.art.w.max(1) as f32;
        // Salen antes las letras más cercanas al monitor de destino.
        let lead = if dx.abs() < 1.0 { self.jit[i] } else if dx > 0.0 { 1.0 - xf } else { xf };
        let s = lead * 0.8 + self.jit[i] * 0.2;
        let d = RELAY_IN + 0.4 + h as f32 * self.hop + s * self.hop * 0.35;
        (d, d + self.hop * 0.55)
    }

    fn hops(&self) -> usize {
        self.route.len() - 1
    }

    fn echo(&self) -> f32 {
        RELAY_IN + 0.4 + self.hops() as f32 * self.hop - self.hop * 0.1 + 0.3
    }
}

impl Effect for Relay {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let me = c.desk.me;
        let v = *c.desk.mine();
        let hops = self.hops();
        let echo = self.echo();
        for i in 0..c.art.cells.len() {
            let (x, y, ch) = c.art.cells[i];
            let base = settle(&c.pal, &c.art, x, y, t);
            let mut shown = None;
            for j in 0..=hops {
                if self.route[j] != me {
                    continue;
                }
                let from = if j == 0 { self.jit[i] * RELAY_IN * 0.7 } else { self.times(c, j - 1, i, x).1 };
                let until = if j == hops { f32::INFINITY } else { self.times(c, j, i, x).0 };
                if t >= from && t < until {
                    shown = Some(from);
                }
            }
            // Al final se enciende en todas las pantallas a la vez.
            let late = echo + self.jit[i] * 0.6;
            if shown.is_none() && t >= late {
                shown = Some(late);
            }
            if let Some(from) = shown {
                let col = shine(flash(base, t - from, 0.55), &c.art, x, y, t, 10.0, 11.8);
                c.g.put(c.x0 + x, c.y0 + y, ch, col);
            }
        }
        // Letras en vuelo, por encima del rótulo.
        for h in 0..hops {
            let (ka, kb) = (self.route[h], self.route[h + 1]);
            let height = c.desk.views[ka].screen.h as f32;
            for i in 0..c.art.cells.len() {
                let (x, y, ch) = c.art.cells[i];
                let (d, a) = self.times(c, h, i, x);
                if t < d || t >= a {
                    continue;
                }
                let (src, dst) = (banner_cell(c, ka, x, y), banner_cell(c, kb, x, y));
                let dist = (dst.0 - src.0).hypot(dst.1 - src.1);
                let lift = if dist < 1.0 { height * 0.9 } else { dist * 0.28 } * (0.85 + 0.3 * self.jit[i]);
                let ctrl = ((src.0 + dst.0) / 2.0, src.1.min(dst.1) - lift * 2.0);
                let at = |p: f32| {
                    let u = ease_in_out_cubic(p);
                    let (m, n) = (lerp2(src, ctrl, u), lerp2(ctrl, dst, u));
                    lerp2(m, n, u)
                };
                let p = (t - d) / (a - d);
                let base = settle(&c.pal, &c.art, x, y, t);
                for k in (1..=4).rev() {
                    let q = p - k as f32 * 0.035;
                    if q >= 0.0 {
                        let (wx, wy) = at(q);
                        put_world(c, &v, wx, wy, if k <= 2 { '•' } else { '·' }, scale(base, 0.8 - 0.15 * k as f32));
                    }
                }
                let (wx, wy) = at(p);
                put_world(c, &v, wx, wy, ch, mix(base, WHITE, 0.55));
            }
        }
    }
}

// ── Cometa ──────────────────────────────────────────────────────────────────
// Un cometa con cola y chispas cruza todas las pantallas de izquierda a derecha pasando por el
// centro de cada una; el rótulo de cada monitor se enciende justo cuando lo atraviesa.

const COMET_T0: f32 = 0.1;
const COMET_TAIL: i32 = 56;
const COMET_TAIL_DT: f32 = 0.012;
const SPARK_EVERY: f32 = 0.025;
const SPARK_LIFE: f32 = 0.8;

struct Comet {
    knots: Vec<(f32, f32)>,
    /// Longitud acumulada hasta cada nudo.
    len: Vec<f32>,
    travel: f32,
    /// Momento en que pasa por el centro del rótulo de cada pantalla, y su dirección.
    pass: Vec<(f32, (f32, f32))>,
    salt: u32,
    amp: f32,
    period: f32,
}

pub fn comet(c: &mut Ctx) -> Box<dyn Effect> {
    let (bx, _, bw, _) = c.desk.bounds;
    let (ux, uy) = c.desk.unit;
    let order = c.desk.order.clone();
    let centers: Vec<(f32, f32)> = order.iter().map(|&k| c.desk.views[k].banner_center(c.art.w, c.art.h)).collect();
    let margin = 3.0 * ux;
    let mut knots = vec![(bx - margin, centers[0].1)];
    knots.extend(&centers);
    knots.push((bx + bw + margin, centers[centers.len() - 1].1));
    let mut len = vec![0.0f32];
    for w in knots.windows(2) {
        len.push(len[len.len() - 1] + (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1).max(1e-3));
    }
    let travel = (1.6 + 1.9 * order.len() as f32).min(8.0);
    let total = len[len.len() - 1];
    let mut pass = vec![(0.0, (1.0, 0.0)); c.desk.views.len()];
    for (j, &k) in order.iter().enumerate() {
        let (a, b) = (knots[j], knots[j + 2]);
        let n = (b.0 - a.0).hypot(b.1 - a.1).max(1e-3);
        pass[k] = (COMET_T0 + len[j + 1] / total * travel, ((b.0 - a.0) / n, (b.1 - a.1) / n));
    }
    let salt = c.rng.next_u64() as u32;
    Box::new(Comet { knots, len, travel, pass, salt, amp: 2.2 * uy, period: 46.0 * ux })
}

impl Comet {
    /// Posición en el escritorio a `tau` segundos de empezar el recorrido (Catmull-Rom).
    fn at(&self, tau: f32) -> (f32, f32) {
        let total = self.len[self.len.len() - 1];
        let s = clamp01(tau / self.travel) * total;
        let j = self.len.windows(2).position(|w| s <= w[1]).unwrap_or(self.len.len() - 2);
        let u = clamp01((s - self.len[j]) / (self.len[j + 1] - self.len[j]).max(1e-3));
        let k = |i: isize| self.knots[i.clamp(0, self.knots.len() as isize - 1) as usize];
        let (p0, p1, p2, p3) = (k(j as isize - 1), k(j as isize), k(j as isize + 1), k(j as isize + 2));
        let cr = |a: f32, b: f32, c: f32, d: f32| {
            0.5 * (2.0 * b + (c - a) * u + (2.0 * a - 5.0 * b + 4.0 * c - d) * u * u + (3.0 * b - a - 3.0 * c + d) * u * u * u)
        };
        let wobble = self.amp * (s / self.period * TAU).sin();
        (cr(p0.0, p1.0, p2.0, p3.0), cr(p0.1, p1.1, p2.1, p3.1) + wobble)
    }
}

impl Effect for Comet {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let v = *c.desk.mine();
        let (ux, uy) = c.desk.unit;
        let speed = self.len[self.len.len() - 1] / self.travel;
        let (tp, dir) = self.pass[c.desk.me];
        for &(x, y, ch) in &c.art.cells {
            let (wx, wy) = v.to_world((v.x0 + x) as f32, (v.y0 + y) as f32);
            let (cx, cy) = v.banner_center(c.art.w, c.art.h);
            let lit = tp + ((wx - cx) * dir.0 + (wy - cy) * dir.1) / speed;
            if t >= lit {
                let base = settle(&c.pal, &c.art, x, y, t);
                let col = shine(flash(base, t - lit, 0.7), &c.art, x, y, t, 8.8, 10.6);
                c.g.put(c.x0 + x, c.y0 + y, ch, col);
            }
        }
        let tau = t - COMET_T0;
        // Chispas: cada una sale de donde estaba el cometa y cae con gravedad.
        let first = ((tau - SPARK_LIFE) / SPARK_EVERY).ceil().max(0.0) as u32;
        let last = (tau.min(self.travel) / SPARK_EVERY).floor();
        if last >= 0.0 {
            for e in first..=last as u32 {
                let te = e as f32 * SPARK_EVERY;
                let age = tau - te;
                if !(0.0..SPARK_LIFE).contains(&age) {
                    continue;
                }
                let (x0, y0) = self.at(te);
                let (vx, vy) = ((hf(e, self.salt, 1) - 0.5) * 14.0 * ux, (hf(e, self.salt, 2) - 0.75) * 9.0 * uy);
                let (wx, wy) = (x0 + vx * age, y0 + vy * age + 0.5 * 24.0 * uy * age * age);
                let k = 1.0 - age / SPARK_LIFE;
                let col = scale(mix(gradient(&c.pal, hf(e, self.salt, 3)), WHITE, 0.3), k);
                put_world(c, &v, wx, wy, if k > 0.5 { '•' } else { '·' }, col);
            }
        }
        // Cola, de la punta más tenue a la cabeza.
        for k in (1..COMET_TAIL).rev() {
            let tk = tau - k as f32 * COMET_TAIL_DT;
            if tk < 0.0 || tk > self.travel {
                continue;
            }
            let q = k as f32 / COMET_TAIL as f32;
            let ch = if q < 0.15 { '▓' } else if q < 0.35 { '▒' } else if q < 0.6 { '•' } else { '·' };
            let col = scale(mix(WHITE, gradient(&c.pal, 0.2 + q * 0.6), q.sqrt()), 1.0 - q * 0.8);
            let (wx, wy) = self.at(tk);
            // Más gruesa cerca de la cabeza; los bordes, más finos y tenues.
            let half = ((1.0 - q) * 2.4) as i32;
            for dy in -half..=half {
                let edge = dy.abs() as f32 / (half as f32 + 1.0);
                let ch = if edge > 0.5 { '·' } else if edge > 0.0 { '•' } else { ch };
                put_world(c, &v, wx, wy + dy as f32 * uy, ch, scale(col, 1.0 - edge * 0.6));
            }
        }
        if (0.0..=self.travel).contains(&tau) {
            let (wx, wy) = self.at(tau);
            let glow = mix(WHITE, c.pal[0], 0.35);
            // Cabeza: un óvalo brillante con halo.
            for dy in -2..=2 {
                for dx in -4..=4 {
                    let r = ((dx as f32 / 3.6).powi(2) + (dy as f32 / 1.8).powi(2)).sqrt();
                    if r > 1.0 {
                        continue;
                    }
                    let ch = if r < 0.35 { '█' } else if r < 0.6 { '▓' } else if r < 0.8 { '▒' } else { '░' };
                    let col = if r < 0.35 { WHITE } else { scale(glow, 1.1 - r * 0.6) };
                    put_world(c, &v, wx + dx as f32 * ux, wy + dy as f32 * uy, ch, col);
                }
            }
        }
    }
}

// ── Marea ───────────────────────────────────────────────────────────────────
// Una ola de bloques recorre el escritorio entero de izquierda a derecha con un color continuo de
// una pantalla a otra y deja el rótulo a su paso; después vuelve, más suave, de derecha a
// izquierda haciendo brillar todos los rótulos.

const TIDE_IN: f32 = 0.05;
const TIDE_BACK: f32 = 6.6;

struct Tide {
    travel: f32,
}

pub fn tide(c: &mut Ctx) -> Box<dyn Effect> {
    Box::new(Tide { travel: (1.4 + 1.0 * c.desk.views.len() as f32).min(4.5) })
}

impl Tide {
    /// Recorrido del frente: entra justo por un borde y sigue hasta que la estela sale por el otro.
    fn ends(c: &Ctx) -> (f32, f32) {
        let (bx, _, bw, _) = c.desk.bounds;
        (bx - 8.0 * c.desk.unit.0, bx + bw + 60.0 * c.desk.unit.0)
    }

    /// Posición del frente de la ola que sale en `start` hacia la derecha (`dir` 1) o la izquierda.
    fn front(&self, c: &Ctx, t: f32, start: f32, dir: f32) -> f32 {
        let (bx, _, bw, _) = c.desk.bounds;
        let (a, b) = Tide::ends(c);
        let p = (t - start) / self.travel;
        if dir > 0.0 {
            a + p * (b - a)
        } else {
            // De vuelta, espejo de la ida.
            2.0 * bx + bw - (a + p * (b - a))
        }
    }
}

/// Intensidad de la ola a `d` celdas por delante (positivo) o por detrás del frente.
fn crest(d: f32) -> f32 {
    if d > 8.0 || d < -60.0 {
        0.0
    } else if d > 0.0 {
        (-(d / 2.5) * (d / 2.5)).exp()
    } else {
        (-(d / 18.0) * (d / 18.0)).exp()
    }
}

impl Effect for Tide {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let v = *c.desk.mine();
        let (bx, _, bw, _) = c.desk.bounds;
        let (ux, uy) = c.desk.unit;
        let f1 = self.front(c, t, TIDE_IN, 1.0);
        let f2 = self.front(c, t, TIDE_BACK, -1.0);
        let back = t >= TIDE_BACK;
        let wave = |wx: f32, wy: f32| {
            let ripple = 3.0 * (wy / uy * 0.22 + t * 2.0).sin();
            let i1 = crest((wx - f1) / ux + ripple);
            let i2 = if back { crest((f2 - wx) / ux + ripple) * 0.55 } else { 0.0 };
            (i1, i2)
        };
        for cy in 0..c.g.rows {
            for cx in 0..c.g.cols {
                let (wx, wy) = v.to_world(cx as f32, cy as f32);
                let (i1, i2) = wave(wx, wy);
                let i = i1.max(i2);
                if i < 0.07 {
                    continue;
                }
                let ch = if i > 0.8 { '█' } else if i > 0.55 { '▓' } else if i > 0.3 { '▒' } else { '░' };
                let col = scale(gradient(&c.pal, (wx - bx) / bw.max(1.0) * 0.8 + t * 0.05), 0.2 + 0.8 * i);
                c.g.put(cx, cy, ch, mix(col, WHITE, 0.35 * i * i));
            }
        }
        let (a, b) = Tide::ends(c);
        for &(x, y, ch) in &c.art.cells {
            let (wx, wy) = v.to_world((v.x0 + x) as f32, (v.y0 + y) as f32);
            let lit = TIDE_IN + (wx - a) / (b - a) * self.travel;
            if t < lit {
                continue;
            }
            let (_, i2) = wave(wx, wy);
            let col = flash(settle(&c.pal, &c.art, x, y, t), t - lit, 0.6);
            c.g.put(c.x0 + x, c.y0 + y, ch, mix(col, WHITE, 0.75 * clamp01(i2 * 1.6)));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::color::Rgb;
    use crate::desk::Screen;
    use crate::fx::{fx_index, EFFECTS};
    use crate::{Config, Engine, WallTime};

    const NOW: WallTime = WallTime { year: 2026, month: 11, day: 10, weekday: 1, hour: 12, minute: 0, second: 0 };

    /// Tres monitores distintos: 1440p a la izquierda, 1080p y uno vertical a la derecha, con un
    /// hueco entre los dos últimos.
    fn desk() -> Vec<Screen> {
        vec![
            Screen { x: -2560.0, y: -180.0, w: 2560.0, h: 1440.0, px_w: 2560, px_h: 1440 },
            Screen::px(1920, 1080),
            Screen { x: 1990.0, y: -400.0, w: 1080.0, h: 1920.0, px_w: 1080, px_h: 1920 },
        ]
    }

    fn engines(id: &str, screens: &[Screen]) -> Vec<Engine> {
        (0..screens.len())
            .map(|me| Engine::with_desk(screens, me, Config::parse(&format!("effects={id}\nlang=es\n"), Rgb::hex(0x0078D4), 5, true)))
            .collect()
    }

    /// Primer momento en que cada pantalla dibuja algo.
    fn first_seen(id: &str, screens: &[Screen]) -> Vec<f64> {
        let mut es = engines(id, screens);
        let mut seen = vec![f64::INFINITY; es.len()];
        for f in 1..(60 * 15) {
            let t = f as f64 / 60.0;
            for (k, e) in es.iter_mut().enumerate() {
                e.tick(t, NOW);
                if seen[k].is_infinite() && e.ctx.g.cells.iter().any(|c| c.fg != 0) {
                    seen[k] = t;
                }
            }
        }
        seen
    }

    #[test]
    fn desk_effects_travel_left_to_right() {
        for id in ["relay", "comet", "tide"] {
            let seen = first_seen(id, &desk());
            // Pantallas 0, 1, 2 de izquierda a derecha: aparece antes en la de la izquierda.
            assert!(seen.iter().all(|s| s.is_finite()), "{id}: {seen:?}");
            assert!(seen[0] < seen[1] && seen[1] < seen[2], "{id}: {seen:?}");
        }
    }

    #[test]
    fn desk_effects_end_with_the_banner_on_every_screen() {
        for id in ["relay", "comet", "tide"] {
            let def = &EFFECTS[fx_index(id).unwrap()];
            let mut es = engines(id, &desk());
            let end = 0.15 + def.secs as f64 - 0.4;
            let mut t = 0.0;
            while t < end {
                t += 1.0 / 30.0;
                for e in es.iter_mut() {
                    e.tick(t, NOW);
                }
            }
            for (k, e) in es.iter().enumerate() {
                assert_eq!(e.current_effect(), Some(id));
                let lit = e.ctx.art.cells.iter().filter(|&&(x, y, ch)| {
                    let cell = e.ctx.g.cells[((e.ctx.y0 + y) * e.ctx.g.cols + e.ctx.x0 + x) as usize];
                    cell.ch == ch && cell.fg != 0
                });
                assert_eq!(lit.count(), e.ctx.art.cells.len(), "{id}: rótulo incompleto en la pantalla {k}");
            }
        }
    }

    #[test]
    fn desk_effects_work_with_one_screen_and_stacked_screens() {
        let stacked = [Screen::px(1920, 1080), Screen { x: 0.0, y: 1080.0, w: 1920.0, h: 1080.0, px_w: 1920, px_h: 1080 }];
        for id in ["relay", "comet", "tide"] {
            for screens in [&[Screen::px(1920, 1080)][..], &stacked, &[Screen::px(320, 240)]] {
                let seen = first_seen(id, screens);
                assert!(seen.iter().all(|s| s.is_finite()), "{id} {screens:?}: {seen:?}");
            }
        }
    }
}
