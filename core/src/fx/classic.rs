//! Los seis efectos del screensaver de Omarchy, con las mismas fórmulas que screensaver-fx.py.
//! El original iba a 30 fps; aquí las velocidades van por segundo y el azar por fotograma se
//! sortea a 30 Hz para que se vea igual a 60 fps o más.

use super::{settle, Ctx, Effect};
use crate::color::{gradient, mix, scale, Rgb, WHITE};
use crate::rng::{hash3, hf};
use crate::Lang;

fn wf(c: &Ctx) -> f32 {
    c.art.w.max(1) as f32
}

// ── Neón ────────────────────────────────────────────────────────────────────

struct Neon {
    on: Vec<f32>,
    flick: Vec<bool>,
}

pub fn neon(c: &mut Ctx) -> Box<dyn Effect> {
    let n = c.art.cells.len();
    let on = (0..n).map(|_| c.rng.range(0.2, 2.2)).collect();
    let flick = (0..n).map(|_| c.rng.chance(0.35)).collect();
    Box::new(Neon { on, flick })
}

impl Effect for Neon {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (w, h) = (wf(c), c.art.h as f32);
        let breath = 0.78 + 0.22 * (t * 2.2).sin();
        let band = ((t - 3.0).rem_euclid(3.2)) / 3.2 * (w + h + 20.0) - 10.0;
        for (i, &(x, y, ch)) in c.art.cells.iter().enumerate() {
            let start = self.on[i];
            let col = if t < start {
                Rgb(18, 18, 22)
            } else if self.flick[i] && t < start + 0.5 && ((t - start) * 20.0) as i32 % 3 == 0 {
                Rgb(40, 40, 48)
            } else {
                let mut col = scale(gradient(&c.pal, x as f32 / w * 0.6 + t * 0.03), breath);
                let d = ((x + y * 2) as f32 - band).abs();
                if t > 3.0 && d < 4.0 {
                    col = mix(col, WHITE, (1.0 - d / 4.0) * 0.85);
                }
                col
            };
            c.g.put(c.x0 + x, c.y0 + y, ch, col);
        }
    }
}

// ── Estrellas ───────────────────────────────────────────────────────────────

struct Starfield {
    stars: Vec<[f32; 3]>,
}

pub fn starfield(c: &mut Ctx) -> Box<dyn Effect> {
    let n = 80.max(c.g.cols * c.g.rows / 25) as usize;
    let stars = (0..n).map(|_| [c.rng.range(-1.0, 1.0), c.rng.range(-1.0, 1.0), c.rng.range(0.1, 1.0)]).collect();
    Box::new(Starfield { stars })
}

impl Effect for Starfield {
    fn draw(&mut self, t: f32, dt: f32, c: &mut Ctx) {
        let (cx, cy) = (c.g.cols as f32 / 2.0, c.g.rows as f32 / 2.0);
        let (bx0, bx1, by0, by1) = (c.x0 - 2, c.x0 + c.art.w + 2, c.y0 - 1, c.y0 + c.art.h + 1);
        for st in self.stars.iter_mut() {
            st[2] -= 0.36 * dt;
            if st[2] <= 0.02 {
                *st = [c.rng.range(-1.0, 1.0), c.rng.range(-1.0, 1.0), 1.0];
            }
            let x = (cx + st[0] / st[2] * cx * 0.5) as i32;
            let y = (cy + st[1] / st[2] * cy * 0.5) as i32;
            if !(0..c.g.cols).contains(&x) || !(0..c.g.rows).contains(&y) {
                *st = [c.rng.range(-1.0, 1.0), c.rng.range(-1.0, 1.0), 1.0];
                continue;
            }
            if (bx0..bx1).contains(&x) && (by0..by1).contains(&y) {
                continue; // el rótulo queda limpio, las estrellas pasan por fuera
            }
            let z = st[2];
            let ch = if z > 0.66 { '·' } else if z > 0.33 { '•' } else { '✶' };
            c.g.put(x, y, ch, scale(WHITE, 1.15 - z));
        }
        let fade = ((t - 1.5) / 2.5).clamp(0.0, 1.0);
        if fade > 0.0 {
            let w = wf(c);
            for &(x, y, ch) in &c.art.cells {
                c.g.put(c.x0 + x, c.y0 + y, ch, scale(gradient(&c.pal, x as f32 / w * 0.6 + t * 0.04), fade));
            }
        }
    }
}

// ── Terminal ────────────────────────────────────────────────────────────────

struct Terminal;

pub fn terminal(_c: &mut Ctx) -> Box<dyn Effect> {
    Box::new(Terminal)
}

const CMD: &str = "whoami";

impl Effect for Terminal {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (px, py) = (c.x0, (c.y0 - 3).max(0));
        let (prompt_c, cmd_c) = (c.pal[0], Rgb(220, 220, 220));
        let n = CMD.len() as i32;
        let typed = (0..n).filter(|&k| t >= 0.8 + k as f32 * 0.13).count() as i32;
        let t_print = 0.8 + n as f32 * 0.13 + 0.4;
        let blink = (t * 2.5) as i32 % 2 == 0;
        c.g.text(px, py, "❯", prompt_c);
        c.g.text(px + 2, py, &CMD[..typed as usize], cmd_c);
        if t < t_print {
            if blink {
                c.g.put(px + 2 + typed, py, '█', cmd_c);
            }
            return;
        }
        let rows_done = c.art.h.min(((t - t_print) / 0.07) as i32 + 1);
        let w = wf(c);
        for &(x, y, ch) in &c.art.cells {
            if y < rows_done {
                c.g.put(c.x0 + x, c.y0 + y, ch, gradient(&c.pal, x as f32 / w * 0.6));
            }
        }
        if rows_done >= c.art.h {
            let end_y = (c.g.rows - 1).min(c.y0 + c.art.h + 2);
            c.g.text(px, end_y, "❯", prompt_c);
            if blink {
                c.g.put(px + 2, end_y, '█', cmd_c);
            }
        }
    }
}

// ── Glitch ──────────────────────────────────────────────────────────────────

const GLITCH: [char; 8] = ['░', '▒', '▓', '█', '▀', '▄', '▌', '▐'];
const SHIFTS: [i32; 6] = [-3, -2, -1, 1, 2, 3];

struct Glitch {
    burst_until: f32,
    next: f32,
    salt: u32,
}

pub fn glitch(c: &mut Ctx) -> Box<dyn Effect> {
    Box::new(Glitch { burst_until: 0.0, next: 1.0, salt: c.rng.next_u64() as u32 })
}

impl Effect for Glitch {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        if t >= self.next {
            self.burst_until = t + c.rng.range(0.15, 0.5);
            self.next = self.burst_until + c.rng.range(0.6, 2.2);
        }
        if t >= self.burst_until {
            for &(x, y, ch) in &c.art.cells {
                c.g.put(c.x0 + x, c.y0 + y, ch, settle(&c.pal, &c.art, x, y, t));
            }
            return;
        }
        let step = (t * 30.0) as u32 ^ self.salt;
        for &(x, y, ch) in &c.art.cells {
            c.g.put(c.x0 + x - 1, c.y0 + y, ch, Rgb(200, 0, 60));
            c.g.put(c.x0 + x + 1, c.y0 + y, ch, Rgb(0, 200, 220));
        }
        for (i, &(x, y, ch)) in c.art.cells.iter().enumerate() {
            let yy = y as u32;
            let shift = if hf(step, yy, 1) < 0.35 { SHIFTS[hash3(step, yy, 2) as usize % 6] } else { 0 };
            let i = i as u32;
            let ch = if hf(step, i, 3) < 0.06 { GLITCH[hash3(step, i, 4) as usize % 8] } else { ch };
            c.g.put(c.x0 + x + shift, c.y0 + y, ch, mix(settle(&c.pal, &c.art, x, y, t), WHITE, 0.3));
        }
    }
}

// ── Plasma ──────────────────────────────────────────────────────────────────

struct Plasma;

pub fn plasma(_c: &mut Ctx) -> Box<dyn Effect> {
    Box::new(Plasma)
}

impl Effect for Plasma {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let fade = (t / 1.5).min(1.0);
        let (hw, hh) = (c.art.w as f32 / 2.0, c.art.h as f32 / 2.0);
        for &(x, y, ch) in &c.art.cells {
            let (xf, yf) = (x as f32, y as f32);
            let v = (xf * 0.12 + t * 1.1).sin()
                + (yf * 0.45 - t * 1.4).sin()
                + ((xf + yf * 2.0) * 0.08 + t * 0.7).sin()
                + ((xf - hw).hypot((yf - hh) * 2.0) * 0.2 - t * 1.6).sin();
            c.g.put(c.x0 + x, c.y0 + y, ch, scale(gradient(&c.pal, v / 8.0 + 0.5 + t * 0.02), fade));
        }
    }
}

// ── Reloj ───────────────────────────────────────────────────────────────────

fn digit(ch: char) -> [&'static str; 4] {
    match ch {
        '0' => ["▄████▄", "██  ██", "██  ██", "▀████▀"],
        '1' => ["▄███  ", "  ██  ", "  ██  ", "██████"],
        '2' => ["▄████▄", "    ██", "▄█▀▀▀▀", "██████"],
        '3' => ["▄████▄", "  ▄▄█▀", "  ▀▀█▄", "▀████▀"],
        '4' => ["██  ██", "██  ██", "▀▀▀▀██", "    ██"],
        '5' => ["██████", "██▄▄▄▄", "▀▀▀▀██", "▀████▀"],
        '6' => ["▄████▄", "██▄▄▄▄", "██▀▀██", "▀████▀"],
        '7' => ["██████", "    ██", "   ██ ", "  ██  "],
        '8' => ["▄████▄", "▀█▄▄█▀", "▄█▀▀█▄", "▀████▀"],
        '9' => ["▄████▄", "██▄▄██", "▀▀▀▀██", "▀████▀"],
        _ => ["  ", "▀▀", "▄▄", "  "],
    }
}

const DIAS: [&str; 7] = ["lunes", "martes", "miércoles", "jueves", "viernes", "sábado", "domingo"];
const MESES: [&str; 12] = ["enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre"];
const DAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

pub fn long_date(lang: Lang, n: &crate::WallTime) -> String {
    let wd = n.weekday.rem_euclid(7) as usize;
    let m = (n.month - 1).clamp(0, 11) as usize;
    match lang {
        Lang::Es => format!("{}, {} de {} de {}", DIAS[wd], n.day, MESES[m], n.year),
        Lang::En => format!("{}, {} {}, {}", DAYS[wd], MONTHS[m], n.day, n.year),
    }
}

struct Clock;

const CLOCK_H: i32 = 4 + 2; // dígitos + línea de fecha
const CLOCK_GAP: i32 = 3;

pub fn clock(c: &mut Ctx) -> Box<dyn Effect> {
    c.y0 = ((c.g.rows - (c.art.h + CLOCK_GAP + CLOCK_H)) / 2).max(0);
    Box::new(Clock)
}

impl Effect for Clock {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let fade = (t / 1.2).min(1.0);
        let w = wf(c);
        for &(x, y, ch) in &c.art.cells {
            c.g.put(c.x0 + x, c.y0 + y, ch, scale(gradient(&c.pal, x as f32 / w * 0.6 + t * 0.03), fade));
        }
        let n = c.now;
        let cy = c.y0 + c.art.h + CLOCK_GAP;
        let hhmm = format!("{:02}:{:02}", n.hour, n.minute);
        let glyphs: Vec<[&str; 4]> = hhmm.chars().map(digit).collect();
        let width: i32 = glyphs.iter().map(|g| g[0].chars().count() as i32).sum::<i32>() + 2 * (glyphs.len() as i32 - 1);
        let mut x = (c.g.cols - width) / 2;
        let dcol = scale(Rgb(235, 235, 235), fade);
        for (ch, g) in hhmm.chars().zip(glyphs.iter()) {
            if ch != ':' || n.second % 2 == 0 {
                for (r, row) in g.iter().enumerate() {
                    c.g.text(x, cy + r as i32, row, dcol);
                }
            }
            x += g[0].chars().count() as i32 + 2;
        }
        let fecha = long_date(c.lang, &n);
        c.g.text((c.g.cols - fecha.chars().count() as i32) / 2, cy + 5, &fecha, scale(c.pal[0], fade));
    }
}
