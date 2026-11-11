//! Efectos temáticos de una pantalla: compilación, circuito, volcado hex, latido, ADN y tinta.

use super::extra::{art_center, dist, smooth};
use super::{clamp01, ease_in_out_cubic, ease_out_cubic, flash, settle, shine, Ctx, Effect};
use crate::color::{gradient, mix, scale, WHITE};
use crate::rng::{hash3, hf};
use core::f32::consts::TAU;

/// Celda de la rejilla donde va la letra `(x, y)` del rótulo.
fn target(c: &Ctx, x: i32, y: i32) -> (f32, f32) {
    ((c.x0 + x) as f32, (c.y0 + y) as f32)
}

// ── Compilación ─────────────────────────────────────────────────────────────

const CRATES: &[&str] = &[
    "ssfx-core", "ssfx-windows", "ssfx-linux", "ssfx-macos", "grid", "palette", "art", "engine", "rand",
    "omarchy", "hyprland", "winit", "tokio", "clap", "serde", "serde_json", "png", "gif", "image", "wgpu",
    "softbuffer", "font8x8", "ini", "dirs", "raw-window-handle", "windows-sys", "objc2", "core-graphics",
    "wayland-client", "x11rb", "notify", "tracing", "anyhow", "thiserror", "memmap2", "time", "byteorder",
    "crc32fast", "flate2", "zstd", "smallvec",
];
const MODULES: &[&str] = &["core", "windows", "linux", "macos", "ffi", "cli", "gui", "build", "examples"];
const WARNINGS: &[&str] = &[
    "unused variable `ch`",
    "unused import: `Rgb`",
    "field `seed` is never read",
    "unused `mut`",
    "function `aux` is never used",
    "constant `MAX` is never used",
];

/// Un registro de compilación sube por la pantalla con su barra de progreso; al terminar,
/// el registro se desvanece y el rótulo aparece letra a letra.
struct Compile {
    /// (texto, tipo) de cada línea: 0 crate, 1 aviso, 2 terminado.
    lines: Vec<(String, u8)>,
    /// Segundos entre líneas.
    rate: f32,
    crates: u32,
}

pub fn compile(c: &mut Ctx) -> Box<dyn Effect> {
    let mut lines: Vec<(String, u8)> = Vec::new();
    let mut warnings = WARNINGS.to_vec();
    for &name in CRATES {
        let verb = if c.rng.chance(0.3) { "Checking" } else { "Compiling" };
        let module = MODULES[c.rng.below(MODULES.len())];
        let ver = format!("v0.{}.{}", c.rng.below(4), c.rng.below(10));
        lines.push((format!("   {verb} {name} {ver} ({module})"), 0));
        if c.rng.chance(0.18) && !warnings.is_empty() {
            let warn = warnings.swap_remove(c.rng.below(warnings.len()));
            lines.push((format!("warning: {warn}"), 1));
        }
    }
    let secs = format!("{}.{}", c.rng.below(3) + 2, c.rng.below(10));
    lines.push((format!("    Finished `release` profile [optimized] target(s) in {secs}s"), 2));
    let rate = 9.2 / lines.len().max(1) as f32;
    Box::new(Compile { lines, rate, crates: CRATES.len() as u32 })
}

impl Compile {
    /// Cuándo termina de salir todo el registro.
    fn end(&self) -> f32 {
        self.rate * self.lines.len() as f32
    }
}

impl Effect for Compile {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (cols, rows) = (c.g.cols, c.g.rows);
        let fade = 1.0 - smooth((t - (self.end() + 0.4)) / 1.2);
        let bottom = rows - 3;
        if fade > 0.0 && cols > 0 {
            let shown = ((t / self.rate).floor() as i32 + 1).clamp(0, self.lines.len() as i32);
            let plain = scale(gradient(&c.pal, 0.55), 0.6);
            for j in 0..shown as usize {
                let y = bottom - (shown - 1 - j as i32);
                if y < 0 {
                    break;
                }
                let (text, kind) = &self.lines[j];
                let col = match kind {
                    1 => gradient(&c.pal, 0.85),
                    2 => mix(gradient(&c.pal, 0.15), WHITE, 0.35),
                    _ => plain,
                };
                c.g.text(2, y, text, scale(col, fade));
            }
            if bottom >= 1 {
                let done = clamp01(t / self.end());
                let built = (done * self.crates as f32) as u32;
                let w = (cols - 16).clamp(10, 34);
                let head = (done * w as f32) as i32;
                let mut bar = String::from("[");
                for i in 0..w {
                    bar.push(if i < head { '=' } else if i == head { '>' } else { ' ' });
                }
                bar.push_str(&format!("] {}/{}", built.min(self.crates), self.crates));
                c.g.text(2, bottom + 1, &bar, scale(gradient(&c.pal, 0.3), fade));
            }
        }
        // Al terminar, el rótulo aparece letra a letra, de izquierda a derecha.
        let start = self.end() + 0.8;
        let n = c.art.cells.len().max(1) as f32;
        for (i, &(x, y, ch)) in c.art.cells.iter().enumerate() {
            let at = start + 1.3 * i as f32 / n;
            if t < at {
                continue;
            }
            let base = settle(&c.pal, &c.art, x, y, t);
            let col = shine(flash(base, t - at, 0.35), &c.art, x, y, t, start + 0.9, start + 1.9);
            c.g.put(c.x0 + x, c.y0 + y, ch, col);
        }
    }
}

// ── Circuito ────────────────────────────────────────────────────────────────

/// Conecta las direcciones hacia la celda anterior y la siguiente con el glifo de pista.
fn glyph(prev: Option<(i32, i32)>, next: Option<(i32, i32)>) -> char {
    let horiz = |d: (i32, i32)| d.1 == 0;
    let side = |d: (i32, i32)| match (d.0.signum(), d.1.signum()) {
        (-1, 0) => 0, // oeste
        (1, 0) => 1,  // este
        (0, -1) => 2, // norte
        _ => 3,       // sur
    };
    match (prev, next) {
        (Some(p), Some(n)) if horiz(p) && horiz(n) => '─',
        (Some(p), Some(n)) if !horiz(p) && !horiz(n) => '│',
        // Esquina: el glifo no depende del orden de llegada y salida.
        (Some(p), Some(n)) => match (side(p).min(side(n)), side(p).max(side(n))) {
            (2, 3) => '│',
            (0, 2) => '└',
            (0, 3) => '┌',
            (1, 2) => '┘',
            _ => '┐',
        },
        (Some(d), None) | (None, Some(d)) => {
            if d.1 == 0 {
                '─'
            } else {
                '│'
            }
        }
        (None, None) => '─',
    }
}

/// Expande una polilínea en celdas con sus glifos de pista.
fn trace_path(way: &[(i32, i32)]) -> Vec<(i32, i32, char)> {
    if way.is_empty() {
        return Vec::new();
    }
    let mut pts = vec![way[0]];
    for w in &way[1..] {
        let (mut x, mut y) = *pts.last().unwrap();
        while (x, y) != *w {
            if x != w.0 {
                x += (w.0 - x).signum();
            } else {
                y += (w.1 - y).signum();
            }
            pts.push((x, y));
        }
    }
    pts.iter()
        .enumerate()
        .map(|(i, &(x, y))| {
            let prev = (i > 0).then(|| (pts[i - 1].0 - x, pts[i - 1].1 - y));
            let next = (i + 1 < pts.len()).then(|| (pts[i + 1].0 - x, pts[i + 1].1 - y));
            (x, y, glyph(prev, next))
        })
        .collect()
}

/// Una pista: camino de celdas con su glifo, inicio y duración del crecimiento.
struct Trace {
    path: Vec<(i32, i32, char)>,
    start: f32,
    dur: f32,
}

/// Pistas de placa que crecen desde los bordes hasta cada columna del rótulo; un pulso viaja por
/// cada pista y, al llegar, enciende las letras de esa columna.
struct Circuit {
    /// Pista por columna de letra.
    tr: Vec<Trace>,
    /// Índice de pista por columna del rótulo.
    col_idx: Vec<usize>,
}

pub fn circuit(c: &mut Ctx) -> Box<dyn Effect> {
    let (cols, rows) = (c.g.cols.max(1), c.g.rows.max(1));
    let mut cols_x: Vec<i32> = c.art.cells.iter().map(|&(x, _, _)| x).collect();
    cols_x.sort_unstable();
    cols_x.dedup();
    let mut tr = Vec::new();
    let mut col_idx = vec![usize::MAX; c.art.w.max(1) as usize];
    for &cx in &cols_x {
        // La pista acaba junto a la letra más alta de la columna, por un lado cualquiera.
        let top = c.art.cells.iter().filter(|&&(x, _, _)| x == cx).map(|&(_, y, _)| y).min().unwrap_or(0);
        let (tx, ty) = (c.x0 + cx, c.y0 + top);
        let end = match c.rng.below(4) {
            0 => (tx, ty - 1),
            1 => (tx, ty + 1),
            2 => (tx - 1, ty),
            _ => (tx + 1, ty),
        };
        // Empieza en un borde de la pantalla y dobla una o dos veces.
        let start = match c.rng.below(4) {
            0 => (0, (c.rng.f32() * rows as f32) as i32),
            1 => (cols - 1, (c.rng.f32() * rows as f32) as i32),
            2 => ((c.rng.f32() * cols as f32) as i32, 0),
            _ => ((c.rng.f32() * cols as f32) as i32, rows - 1),
        };
        let way = if c.rng.chance(0.5) {
            vec![start, (end.0, start.1), end]
        } else {
            let mid = (c.rng.f32() * cols as f32) as i32;
            vec![start, (mid, start.1), (mid, end.1), end]
        };
        let path = trace_path(&way);
        let dur = c.rng.range(2.0, 4.0);
        let start_t = c.rng.range(0.3, 1.8);
        col_idx[cx as usize] = tr.len();
        tr.push(Trace { path, start: start_t, dur });
    }
    Box::new(Circuit { tr, col_idx })
}

impl Effect for Circuit {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        for tr in &self.tr {
            let grown = tr.path.len() as f32 * clamp01((t - tr.start) / tr.dur);
            if grown <= 0.0 {
                continue;
            }
            // Al llegar el pulso, toda la pista se enciende un instante y vuelve a su brillo.
            let arrive = tr.start + tr.dur;
            let pulse = if t >= arrive { 1.0 - clamp01((t - arrive) / 1.1) } else { 0.0 };
            let base = gradient(&c.pal, 0.3 + 0.4 * grown / tr.path.len().max(1) as f32);
            for (i, &(x, y, ch)) in tr.path.iter().enumerate() {
                if i as f32 >= grown {
                    break;
                }
                let col = if grown - (i as f32) < 1.5 {
                    mix(base, WHITE, 0.75)
                } else {
                    scale(base, 0.35 + 0.6 * pulse)
                };
                c.g.put(x, y, ch, col);
            }
        }
        for &(x, y, ch) in &c.art.cells {
            let idx = self.col_idx[x as usize];
            let Some(tr) = self.tr.get(idx) else { continue };
            let arrive = tr.start + tr.dur;
            let (tx, ty) = (c.x0 + x, c.y0 + y);
            let base = settle(&c.pal, &c.art, x, y, t);
            if t < arrive {
                // La silueta apenas se intuye bajo la placa.
                c.g.put(tx, ty, ch, scale(base, 0.12));
            } else {
                let col = shine(flash(base, t - arrive, 0.45), &c.art, x, y, t, 10.2, 11.8);
                c.g.put(tx, ty, ch, col);
            }
        }
    }
}

// ── Volcado hex ─────────────────────────────────────────────────────────────

const HEXD: &[u8; 16] = b"0123456789abcdef";
/// Ancho de un bloque de volcado: 8 de offset + 2 de hueco + 48 de hex + 1 de hueco + 16 ASCII + 1.
const BLOCK: i32 = 76;
/// Cambios de celda por segundo: con 60 fps solo una fracción del volcado cambia en cada cuadro.
const SCRAMBLE: f32 = 2.5;

/// Las columnas de un volcado hexadecimal llenan la pantalla, con bloques de offset + 16 bytes +
/// ASCII uno al lado de otro; cada celda cambia en su propio momento y las que caen sobre el
/// rótulo se descodifican, de dígito hex a la letra real.
struct Hexdump {
    seed: u32,
    /// Fase de cambio por celda (en unidades de paso): cuándo salta su contenido.
    phase: Vec<f32>,
    /// (paso, carácter, clase de brillo) por celda: el hash solo se repite si el paso cambió.
    cache: Vec<(u32, char, u8)>,
    size: (i32, i32),
}

pub fn hexdump(c: &mut Ctx) -> Box<dyn Effect> {
    let mut fx = Hexdump { seed: c.rng.next_u64() as u32, phase: Vec::new(), cache: Vec::new(), size: (-1, -1) };
    fx.fit(c.g.cols, c.g.rows);
    Box::new(fx)
}

impl Hexdump {
    /// (Re)calcula las fases por celda si cambió el tamaño de la rejilla.
    fn fit(&mut self, cols: i32, rows: i32) {
        if self.size == (cols, rows) {
            return;
        }
        self.size = (cols, rows);
        let n = (cols.max(0) * rows.max(0)) as usize;
        self.phase.clear();
        self.phase.resize(n, 0.0);
        self.cache.clear();
        self.cache.resize(n, (u32::MAX, ' ', 0));
        for gy in 0..rows.max(0) {
            for gx in 0..cols.max(0) {
                self.phase[(gy * cols + gx) as usize] = hf(gx as u32, gy as u32, self.seed) * 1024.0;
            }
        }
    }

    /// (carácter, clase de brillo) del volcado en la celda de pantalla `(gx, gy)` en el paso
    /// `slot`: 0 hueco, 1 offset, 2 hex, 3 ASCII.
    fn glyph(&self, gx: i32, gy: i32, slot: u32) -> (char, u8) {
        let row = gy.max(0) as u32;
        let lx = gx.rem_euclid(BLOCK);
        if lx < 8 {
            let v = (row.wrapping_mul(16) >> (4 * (7 - lx))) & 0xF;
            (HEXD[v as usize] as char, 1)
        } else if (10..58).contains(&lx) {
            let (k, r) = ((lx - 10) / 3, (lx - 10) % 3);
            if r == 2 {
                (' ', 0)
            } else {
                let b = hash3(self.seed ^ row, k as u32, slot) as u8;
                (HEXD[(if r == 0 { b >> 4 } else { b & 0xF }) as usize] as char, 2)
            }
        } else if (59..75).contains(&lx) {
            let b = hash3(self.seed ^ row, (lx - 59) as u32, slot) as u8;
            let ch = if (32..127).contains(&b) { b as char } else { '·' };
            (ch, 3)
        } else {
            (' ', 0)
        }
    }
}

impl Effect for Hexdump {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let fade = clamp01(t / 0.4) * (1.0 - smooth((t - 8.2) / 1.6));
        if fade > 0.0 {
            let off = scale(gradient(&c.pal, 0.3 + 0.25 * 0.35), 0.35 * fade);
            let hex = scale(gradient(&c.pal, 0.3 + 0.25 * 0.6), 0.6 * fade);
            let ascii = scale(gradient(&c.pal, 0.3 + 0.25 * 0.8), 0.8 * fade);
            self.fit(c.g.cols, c.g.rows);
            let (cols, rows) = (c.g.cols, c.g.rows);
            let tt = t * SCRAMBLE;
            for gy in 0..rows {
                for gx in 0..cols {
                    let i = (gy * cols + gx) as usize;
                    // Tiempo cuantizado con fase propia por celda: el contenido solo cambia
                    // cuando el paso salta, así que una fracción de celdas se repinta por cuadro.
                    let slot = (tt + self.phase[i]) as u32;
                    let (ch, cl) = if self.cache[i].0 == slot {
                        (self.cache[i].1, self.cache[i].2)
                    } else {
                        let g = self.glyph(gx, gy, slot);
                        self.cache[i] = (slot, g.0, g.1);
                        g
                    };
                    let col = match cl {
                        3 => ascii,
                        2 => hex,
                        1 => off,
                        _ => continue,
                    };
                    c.g.put(gx, gy, ch, col);
                }
            }
        }
        // Cada letra se descodifica al llegar su turno, de izquierda a derecha.
        let w = c.art.w.max(1) as f32;
        for &(x, y, ch) in &c.art.cells {
            let decode = 3.2 + 4.6 * x as f32 / w;
            if t < decode {
                continue;
            }
            let base = settle(&c.pal, &c.art, x, y, t);
            let col = shine(flash(base, t - decode, 0.4), &c.art, x, y, t, 9.0, 10.8);
            c.g.put(c.x0 + x, c.y0 + y, ch, col);
        }
    }
}

// ── Latido ──────────────────────────────────────────────────────────────────

const SWEEP: f32 = 10.6;

/// La forma de un latido (ondas P-QRS-T) con la fase `u` del ciclo.
fn ecg(u: f32) -> f32 {
    let g = |c: f32, w: f32, h: f32| h * (-((u - c) / w).powi(2)).exp();
    g(0.14, 0.045, 0.35) - g(0.255, 0.012, 0.25) + g(0.29, 0.016, 1.6) - g(0.32, 0.014, 0.45) + g(0.52, 0.07, 0.5)
}

/// Un electrocardiograma se dibuja de izquierda a derecha a la altura del rótulo; cada latido que
/// pasa por una columna la revela.
struct Heartbeat;

pub fn heartbeat(_c: &mut Ctx) -> Box<dyn Effect> {
    Box::new(Heartbeat)
}

impl Effect for Heartbeat {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (cols, rows) = (c.g.cols, c.g.rows);
        let cy = (c.y0 + c.art.h / 2) as f32;
        // Rejilla tenue de fondo.
        let grid_col = scale(gradient(&c.pal, 0.5), 0.16);
        for gy in (0..rows).step_by(2) {
            for gx in (0..cols).step_by(3) {
                c.g.put(gx, gy, '·', grid_col);
            }
        }
        let head = t / SWEEP * cols.max(1) as f32;
        let speed = cols.max(1) as f32 / SWEEP;
        let beats = cols.max(1) as f32 / 24.0;
        // Picos QRS de ±4-5 filas.
        let amp = 3.0f32;
        // La estela cubre al menos el 60% del ancho y se apaga gradualmente de cabeza a cola.
        let trail = (cols.max(1) as f32 * 0.62).max(28.0);
        let fade = 1.0 - smooth((t - (SWEEP + 0.3)) / 1.2);
        let first = (head - trail).max(0.0) as i32;
        let last = ((head as i32).min(cols - 1)).max(first - 1);
        for gx in first..=last {
            let u = (gx as f32 / cols.max(1) as f32 * beats).fract();
            let prev = (gx as f32 - 1.0) / cols.max(1) as f32 * beats;
            let y = cy - ecg(u) * amp;
            let age = (head - gx as f32).max(0.0);
            let k = clamp01(1.0 - age / trail);
            let col = scale(mix(gradient(&c.pal, 0.7), WHITE, 0.2 + 0.8 * k * k), (0.15 + 0.85 * k) * fade);
            // Une los saltos bruscos de la aguja (QRS) con una línea vertical; el trazo plano
            // queda como una línea continua de bloques.
            let prev_y = cy - ecg(prev.fract()) * amp;
            let (lo, hi) = (prev_y.min(y).round() as i32, prev_y.max(y).round() as i32);
            for yy in lo..=hi {
                c.g.put(gx, yy, if lo == hi { '█' } else { '│' }, col);
            }
            if age <= 1.5 {
                let ch = if k > 0.75 { '█' } else if k > 0.45 { '▓' } else if k > 0.2 { '▒' } else { '░' };
                c.g.put(gx, y.round() as i32, ch, col);
            }
        }
        if fade > 0.0 && (0.0..cols as f32).contains(&head) {
            let u = (head / cols.max(1) as f32 * beats).fract();
            c.g.put(head as i32, (cy - ecg(u) * amp).round() as i32, '█', WHITE);
        }
        // Las columnas que la traza ya ha cruzado quedan reveladas.
        for &(x, y, ch) in &c.art.cells {
            let tx = c.x0 + x;
            let base = settle(&c.pal, &c.art, x, y, t);
            if tx as f32 <= head {
                let since = (head - tx as f32) / speed;
                let col = shine(flash(base, since, 0.4), &c.art, x, y, t, 10.0, 11.8);
                c.g.put(tx, c.y0 + y, ch, col);
            }
        }
    }
}

// ── ADN ─────────────────────────────────────────────────────────────────────

/// Una doble hélice horizontal gira centrada en el rótulo; luego se abre, sus peldaños se
/// convierten en las letras y la hélice se desvanece.
struct Dna {
    phase: f32,
}

pub fn dna(c: &mut Ctx) -> Box<dyn Effect> {
    Box::new(Dna { phase: c.rng.range(0.0, TAU) })
}

impl Effect for Dna {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        let (cols, rows) = (c.g.cols, c.g.rows);
        let (_, cy) = art_center(c);
        let fade = clamp01(t / 0.8) * (1.0 - smooth((t - 9.8) / 2.0));
        let open = smooth((t - 7.6) / 2.2);
        if fade > 0.0 {
            let lambda = 26.0f32;
            let spin = t * 1.4 + self.phase;
            let amp = c.art.h as f32 * 0.55 + 1.0;
            let sep = open * (rows as f32 * 0.22 + 2.0);
            for gx in 0..cols {
                let s = (gx as f32 * TAU / lambda + spin).sin();
                let z = (gx as f32 * TAU / lambda + spin).cos();
                let (y1, y2) = (cy - s * amp - sep, cy + s * amp + sep);
                // Peldaño: brillo según la profundidad de la hebra que lo sostiene.
                if gx % 3 == 0 {
                    let b = (0.25 + 0.45 * z.abs()) * fade;
                    let ch = if z.abs() > 0.5 { '│' } else { '·' };
                    let (lo, hi) = (y1.min(y2).round() as i32, y1.max(y2).round() as i32);
                    for yy in lo..=hi {
                        c.g.put(gx, yy, ch, scale(gradient(&c.pal, 0.55), b));
                    }
                }
                for (y, zz) in [(y1, z), (y2, -z)] {
                    let col = scale(gradient(&c.pal, 0.15 + 0.3 * (zz + 1.0) * 0.5), (0.35 + 0.55 * zz.abs()) * fade);
                    c.g.put(gx, y.round() as i32, if zz > 0.0 { '█' } else { '▓' }, col);
                }
            }
        }
        // Los peldaños se convierten en letras, de izquierda a derecha.
        let w = c.art.w.max(1) as f32;
        for &(x, y, ch) in &c.art.cells {
            let at = 8.2 + 1.8 * x as f32 / w;
            if t < at {
                continue;
            }
            let base = settle(&c.pal, &c.art, x, y, t);
            let col = shine(flash(base, t - at, 0.45), &c.art, x, y, t, 10.6, 12.6);
            c.g.put(c.x0 + x, c.y0 + y, ch, col);
        }
    }
}

// ── Tinta ───────────────────────────────────────────────────────────────────

const FALL: f32 = 0.5;
/// Segundos que tarda la mancha en expandirse hasta su tamaño final.
const SPREAD: f32 = 1.5;

/// (punto de impacto, inicio de la caída, colores, tamaño) por gota.
struct Drop {
    x: f32,
    y: f32,
    start: f32,
    ca: f32,
    cb: f32,
    size: f32,
    id: u32,
}

/// Instante en que la mancha de `d` cubre la celda `(tx, ty)`, si la llega a cubrir.
fn cover_at(d: &Drop, tx: f32, ty: f32, seed: u32) -> Option<f32> {
    let (ddx, ddy) = (tx - d.x, ty - d.y);
    // Borde irregular en el ángulo de la celda, con la misma regla que al dibujar.
    let bucket = ((ddy * 2.0).atan2(ddx) / TAU * 12.0).round() as u32;
    let edge = d.size * (0.62 + 0.55 * hf(d.id, bucket, seed));
    let q = dist(ddx, ddy) / edge;
    if q >= 1.0 {
        return None;
    }
    // r = size * ease_out_cubic(p): se despeja p de q = ease_out_cubic(p).
    let p = 1.0 - (1.0 - q).cbrt();
    Some(d.start + FALL + SPREAD * p)
}

/// Gotas de tinta caen sobre el papel y se expanden como manchas de acuarela; las que cubren el
/// rótulo dejan ver las letras, y al secarse solo queda el rótulo.
struct Ink {
    drops: Vec<Drop>,
    /// Momento en que la tinta deja ver cada letra.
    reveal: Vec<f32>,
    seed: u32,
    /// Puntos del grano del papel, calculados una vez por tamaño de rejilla.
    paper: Vec<(i32, i32)>,
    size: (i32, i32),
    /// Forma de cada mancha: celdas relativas a la gota con su distancia normalizada al borde
    /// (la mancha de radio `r` cubre las que tienen `n <= r`), ordenadas de dentro afuera.
    shapes: Vec<Vec<(i8, i8, f32)>>,
}

/// Celdas de la mancha de `d` a su tamaño final, con la misma regla de borde que `cover_at`.
fn shape(d: &Drop, seed: u32) -> Vec<(i8, i8, f32)> {
    let (bx, by) = (d.x.floor(), d.y.floor());
    let lim = (d.size * 1.2).ceil() as i32 + 1;
    let mut v = Vec::new();
    for oy in -lim..=lim {
        for ox in -lim..=lim {
            let (ddx, ddy) = (bx + ox as f32 + 0.5 - d.x, by + oy as f32 + 0.5 - d.y);
            let bucket = ((ddy * 2.0).atan2(ddx) / TAU * 12.0).round() as u32;
            let n = dist(ddx, ddy) / (0.62 + 0.55 * hf(d.id, bucket, seed));
            if n <= d.size {
                v.push((ox as i8, oy as i8, n));
            }
        }
    }
    v.sort_unstable_by(|a, b| a.2.total_cmp(&b.2));
    v
}

pub fn ink(c: &mut Ctx) -> Box<dyn Effect> {
    let seed = c.rng.next_u64() as u32;
    // 10-16 gotas; la mitad apunta cerca del rótulo, el resto cae en cualquier parte del papel.
    let n = c.rng.range(10.0, 17.0) as i32;
    let mut drops: Vec<Drop> = (0..n)
        .map(|k| {
            let near = k % 2 == 0;
            let x = if near { c.x0 as f32 + c.rng.range(0.0, c.art.w.max(1) as f32) } else { c.rng.range(0.0, c.g.cols.max(1) as f32) };
            let y = if near { c.y0 as f32 + c.rng.range(0.0, c.art.h.max(1) as f32) } else { c.rng.range(0.0, c.g.rows.max(1) as f32) };
            Drop {
                x,
                y,
                start: 0.3 + k as f32 * 0.22 + c.rng.range(0.0, 0.3),
                ca: c.rng.f32(),
                cb: c.rng.f32(),
                size: if near { c.rng.range(8.0, 20.0) } else { c.rng.range(4.0, 8.0) },
                id: k as u32,
            }
        })
        .collect();
    // Hacia la mitad del efecto el rótulo entero está cubierto: donde ninguna gota llega,
    // cae una extra sobre la letra.
    let mut id = drops.len() as u32;
    for &(x, y, _) in &c.art.cells {
        let (tx, ty) = target(c, x, y);
        let covered = drops.iter().any(|d| dist(tx - d.x, ty - d.y) <= d.size * 0.9);
        if !covered && id < 48 {
            drops.push(Drop {
                x: tx,
                y: ty,
                start: c.rng.range(0.5, 4.0),
                ca: c.rng.f32(),
                cb: c.rng.f32(),
                size: c.rng.range(9.0, 16.0),
                id,
            });
            id += 1;
        }
    }
    // Cada letra se revela en el instante en que la mancha que la cubre la alcanza; si ninguna
    // la toca, la deja ver el secado final.
    let reveal = c
        .art
        .cells
        .iter()
        .map(|&(x, y, _)| {
            let (tx, ty) = target(c, x, y);
            drops.iter().filter_map(|d| cover_at(d, tx, ty, seed)).fold(9.2f32, f32::min)
        })
        .collect();
    let shapes = drops.iter().map(|d| shape(d, seed)).collect();
    let mut fx = Ink { drops, reveal, seed, paper: Vec::new(), size: (-1, -1), shapes };
    fx.paper_for(c.g.cols, c.g.rows);
    Box::new(fx)
}

impl Ink {
    /// Recalcula el grano del papel si cambió el tamaño de la rejilla.
    fn paper_for(&mut self, cols: i32, rows: i32) {
        if self.size == (cols, rows) {
            return;
        }
        self.size = (cols, rows);
        self.paper.clear();
        for gy in 0..rows {
            for gx in 0..cols {
                if hf(gx as u32, gy as u32, self.seed) > 0.965 {
                    self.paper.push((gx, gy));
                }
            }
        }
    }
}

impl Effect for Ink {
    fn draw(&mut self, t: f32, _dt: f32, c: &mut Ctx) {
        self.paper_for(c.g.cols, c.g.rows);
        // El papel: una textura apenas visible.
        for &(gx, gy) in &self.paper {
            c.g.put(gx, gy, '·', scale(gradient(&c.pal, 0.6), 0.1));
        }
        // Al secarse, la tinta se aclara hasta desaparecer.
        let dry = 1.0 - smooth((t - 8.6) / 2.6);
        for (i, d) in self.drops.iter().enumerate() {
            let impact = d.start + FALL;
            if t < impact {
                // La gota cae con un pequeño brillo de superficie mojada.
                let y = d.y * ease_in_out_cubic((t - d.start) / FALL) - 1.0;
                c.g.put(d.x as i32, y as i32, '█', mix(gradient(&c.pal, d.ca), WHITE, 0.4));
                continue;
            }
            let r = d.size * ease_out_cubic((t - impact) / SPREAD);
            if r <= 0.05 || dry <= 0.0 {
                continue;
            }
            let tint = mix(gradient(&c.pal, d.ca), gradient(&c.pal, d.cb), 0.5);
            // Borde irregular ya calculado: la mancha crece recorriendo su forma de dentro afuera.
            let (bx, by) = (d.x.floor() as i32, d.y.floor() as i32);
            for &(ox, oy, n) in &self.shapes[i] {
                if n > r {
                    break;
                }
                let k = 1.0 - n / r;
                let (ch, v) = if k > 0.6 {
                    ('▓', 0.6 + 0.4 * k)
                } else if k > 0.25 {
                    ('▒', 0.38 + 0.5 * k)
                } else {
                    ('░', 0.18 + 0.45 * k)
                };
                c.g.put(bx + ox as i32, by + oy as i32, ch, scale(tint, v * dry));
            }
        }
        // Las letras, reveladas por la tinta que las cubre (o por el secado).
        for (i, &(x, y, ch)) in c.art.cells.iter().enumerate() {
            if t < self.reveal[i] {
                continue;
            }
            let base = settle(&c.pal, &c.art, x, y, t);
            let col = shine(flash(base, t - self.reveal[i], 0.5), &c.art, x, y, t, 10.0, 11.8);
            c.g.put(c.x0 + x, c.y0 + y, ch, col);
        }
    }
}
