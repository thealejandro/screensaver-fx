//! Graba cada efecto completo en un GIF animado para la galería del README.
//! cargo run -p ssfx-core --example gif --release -- [carpeta] [ancho] [alto] [fps] [velocidad] [ini extra] [efectos]
//! Por defecto 760x432 (lo mínimo del motor) a 8 fps y velocidad 2 (el efecto entero, acelerado). El tamaño
//! se elige para que el búfer quede a escala 1: los glifos salen nítidos, sin reescalar.

use ssfx_core::color::Rgb;
use ssfx_core::fx::EFFECTS;
use ssfx_core::{Config, Engine, WallTime};
use std::fs::File;

/// Tiempo que se queda quieto el último cuadro antes de repetir, en centésimas.
const HOLD: u16 = 120;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |i: usize| args.get(i).filter(|s| !s.is_empty());
    let dir = arg(1).cloned().unwrap_or_else(|| "gifs".into());
    let w: i32 = arg(2).and_then(|s| s.parse().ok()).unwrap_or(760);
    let h: i32 = arg(3).and_then(|s| s.parse().ok()).unwrap_or(432);
    let fps: f64 = arg(4).and_then(|s| s.parse().ok()).unwrap_or(8.0);
    let speed: f64 = arg(5).and_then(|s| s.parse().ok()).unwrap_or(2.0);
    let extra = arg(6).map(|s| s.replace('|', "\n")).unwrap_or_else(|| "banner=text\ntitle=FX\nsubtitle=SCREENSAVER".into());
    let only: Vec<&str> = arg(7).map(|s| s.split(',').map(str::trim).collect()).unwrap_or_default();
    std::fs::create_dir_all(&dir).unwrap();
    let now = WallTime { year: 2026, month: 11, day: 9, weekday: 0, hour: 21, minute: 47, second: 10 };
    let delay = (100.0 / fps).round() as u16;
    for def in EFFECTS.iter() {
        let id = def.id_str();
        if !only.is_empty() && !only.contains(&id) {
            continue;
        }
        let ini = format!("theme=tokyo-night\neffects={id}\nlang=es\n{extra}\n");
        let mut e = Engine::new(w, h, Config::parse(&ini, Rgb::hex(0x0078D4), 7, true));
        let l = e.layout();
        if l.int_scale() != Some(1) {
            eprintln!("aviso: {id} no queda a escala 1 ({}x{} en {w}x{h})", l.buf_w, l.buf_h);
        }
        let (bw, bh) = (l.buf_w as usize, l.buf_h as usize);
        let path = format!("{dir}/{id}.gif");
        let mut enc = gif::Encoder::new(File::create(&path).unwrap(), bw as u16, bh as u16, &[]).unwrap();
        enc.set_repeat(gif::Repeat::Infinite).unwrap();
        // El hueco inicial (0,15 s) y el efecto entero; el reloj del motor avanza a 60 Hz.
        let end = 0.15 + def.secs as f64;
        let (mut t, mut next, mut frames) = (0.0f64, 0.15f64, 0);
        let mut prev: Vec<u32> = vec![u32::MAX; bw * bh];
        while next <= end {
            while t < next {
                t += 1.0 / 60.0;
                e.tick(t, now);
            }
            next += speed / fps;
            let last = next > end;
            let px = e.pixels();
            // Solo el rectángulo que cambió; lo que sigue igual queda transparente.
            let (mut x0, mut y0, mut x1, mut y1) = (bw, bh, 0, 0);
            for (i, (&a, &b)) in px.iter().zip(&prev).enumerate() {
                if a != b {
                    let (x, y) = (i % bw, i / bw);
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
                }
            }
            if x0 >= x1 {
                (x0, y0, x1, y1) = (0, 0, 1, 1);
            }
            let (fw, fh) = (x1 - x0, y1 - y0);
            let mut rgba = Vec::with_capacity(fw * fh * 4);
            for y in y0..y1 {
                for x in x0..x1 {
                    let (p, q) = (px[y * bw + x], prev[y * bw + x]);
                    let a = if p == q { 0 } else { 255 };
                    rgba.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, a]);
                }
            }
            let mut f = gif::Frame::from_rgba_speed(fw as u16, fh as u16, &mut rgba, 10);
            (f.left, f.top) = (x0 as u16, y0 as u16);
            f.dispose = gif::DisposalMethod::Keep;
            f.delay = if last { HOLD } else { delay };
            enc.write_frame(&f).unwrap();
            prev.copy_from_slice(px);
            frames += 1;
        }
        drop(enc);
        let kb = std::fs::metadata(&path).map(|m| m.len() / 1024).unwrap_or(0);
        println!("{id:<12} {frames:>4} cuadros {kb:>5} KB  {bw}x{bh}");
    }
}
