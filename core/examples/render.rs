//! Vuelca fotogramas del motor real como RGB en bruto (rgb24) por la salida estándar, para
//! montar vídeos con ffmpeg sin ventana. Lo usa promo/scripts/clips.py.
//!
//! cargo run -p ssfx-core --example render --release -- <ancho> <alto> <fps> <desde> <hasta> [ini] [--info]
//!
//! - `ancho` y `alto`: pantalla simulada en píxeles; el motor elige la rejilla y el búfer.
//! - `desde` y `hasta`: segundos del motor (el primer efecto empieza a los 0,15 s). Se avanza
//!   fotograma a fotograma desde 0 y solo se escriben los de `[desde, hasta)`.
//! - `ini`: líneas de config.ini separadas por `|` (`theme=aero-night|effects=plasma`). Claves
//!   propias de esta herramienta: `seed` (7), `accent` (hex, 0078d4), `now` (hora de pared
//!   inicial `AAAA-MM-DD HH:MM:SS`, avanza con el tiempo), `weekday` (0 = lunes) y `art`
//!   (ruta de un screensaver.txt para `banner=file`).
//! - `--info`: solo escribe `buf_w buf_h dst_x dst_y dst_w dst_h cols rows` y sale.
//!
//! Cada fotograma ocupa `buf_w * buf_h * 3` bytes. Para la pantalla completa:
//! ffmpeg -f rawvideo -pix_fmt rgb24 -s <buf_w>x<buf_h> -r <fps> -i - \
//!   -vf "scale=<dst_w>:<dst_h>:flags=neighbor,pad=<ancho>:<alto>:<dst_x>:<dst_y>" ...

use std::io::{BufWriter, Write};

use ssfx_core::color::Rgb;
use ssfx_core::{Config, Engine, WallTime};

fn arg<T: std::str::FromStr>(args: &[String], i: usize, name: &str) -> T {
    args.get(i).and_then(|s| s.parse().ok()).unwrap_or_else(|| {
        eprintln!("uso: render <ancho> <alto> <fps> <desde> <hasta> [ini] [--info] (falta o no vale: {name})");
        std::process::exit(2)
    })
}

/// Valor de una clave propia de la herramienta dentro del ini.
fn key<'a>(ini: &'a str, k: &str) -> Option<&'a str> {
    ini.lines().filter_map(|l| l.split_once('=')).find(|(a, _)| a.trim().eq_ignore_ascii_case(k)).map(|(_, v)| v.trim())
}

fn parse_now(s: &str, weekday: i32) -> Option<WallTime> {
    let (d, t) = s.split_once(' ')?;
    let d: Vec<i32> = d.split('-').map(|x| x.parse().ok()).collect::<Option<_>>()?;
    let t: Vec<i32> = t.split(':').map(|x| x.parse().ok()).collect::<Option<_>>()?;
    if d.len() != 3 || t.len() != 3 {
        return None;
    }
    Some(WallTime { year: d[0], month: d[1], day: d[2], weekday, hour: t[0], minute: t[1], second: t[2] })
}

/// La hora de pared avanza con el tiempo del motor (sin cambiar de día: basta para un clip).
fn advance(base: WallTime, secs: i64) -> WallTime {
    let total = (base.hour as i64 * 3600 + base.minute as i64 * 60 + base.second as i64 + secs).rem_euclid(86_400);
    WallTime { hour: (total / 3600) as i32, minute: (total / 60 % 60) as i32, second: (total % 60) as i32, ..base }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).filter(|a| a != "--info").collect();
    let info = std::env::args().any(|a| a == "--info");
    let w: i32 = arg(&args, 0, "ancho");
    let h: i32 = arg(&args, 1, "alto");
    let fps: f64 = arg(&args, 2, "fps");
    let from: f64 = arg(&args, 3, "desde");
    let to: f64 = arg(&args, 4, "hasta");
    let ini = args.get(5).map(|s| s.replace('|', "\n")).unwrap_or_default();

    let seed = key(&ini, "seed").and_then(|v| v.parse().ok()).unwrap_or(7);
    let accent = key(&ini, "accent").and_then(|v| u32::from_str_radix(v.trim_start_matches('#'), 16).ok()).unwrap_or(0x0078D4);
    let weekday = key(&ini, "weekday").and_then(|v| v.parse().ok()).unwrap_or(0);
    let now = key(&ini, "now").and_then(|v| parse_now(v, weekday)).unwrap_or(WallTime { year: 2026, month: 11, day: 9, weekday: 0, hour: 21, minute: 47, second: 10 });

    let mut cfg = Config::parse(&ini, Rgb::hex(accent), seed, !ini.contains("lang=en"));
    if let Some(path) = key(&ini, "art") {
        // Con `banner=file`: el contenido de screensaver.txt sale de este archivo.
        cfg.art = std::fs::read_to_string(path).ok().filter(|s| !s.trim().is_empty());
    }
    let mut e = Engine::new(w, h, cfg);
    let l = e.layout();
    if info {
        println!("{} {} {} {} {} {} {} {}", l.buf_w, l.buf_h, l.dst_x, l.dst_y, l.dst_w, l.dst_h, l.cols, l.rows);
        return;
    }

    let mut out = BufWriter::with_capacity(1 << 20, std::io::stdout().lock());
    let mut rgb = vec![0u8; (l.buf_w * l.buf_h * 3) as usize];
    let mut i = 0u64;
    loop {
        let t = i as f64 / fps;
        if t >= to {
            break;
        }
        e.tick(t, advance(now, t as i64));
        if t >= from {
            for (px, dst) in e.pixels().iter().zip(rgb.chunks_exact_mut(3)) {
                dst[0] = (px >> 16) as u8;
                dst[1] = (px >> 8) as u8;
                dst[2] = *px as u8;
            }
            if out.write_all(&rgb).is_err() {
                return; // ffmpeg cerró la tubería
            }
        }
        i += 1;
    }
    let _ = out.flush();
}
