//! Renderiza fotogramas de cada efecto a PNG para revisarlos sin ventana.
//! cargo run -p ssfx-core --example snap --release -- [carpeta] [tema] [ancho] [alto] [ini extra] [momentos] [efectos]
//! momentos: fracciones de la duración separadas por comas (por defecto 0.25,0.5,0.97).
//! efectos: ids separados por comas (por defecto, todos).

#[path = "png.rs"]
mod png;

use ssfx_core::color::Rgb;
use ssfx_core::fx::EFFECTS;
use ssfx_core::{Config, Engine, WallTime};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = args.get(1).cloned().unwrap_or_else(|| "snaps".into());
    let theme = args.get(2).cloned().unwrap_or_else(|| "auto".into());
    let w: i32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1920);
    let h: i32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1080);
    // 5.º argumento: líneas extra de ini separadas por '|', p. ej. "banner=text|title=Hola".
    let extra = args.get(5).map(|s| s.replace('|', "\n")).unwrap_or_default();
    let fracs: Vec<f32> = args.get(6).map(|s| s.split(',').filter_map(|v| v.trim().parse().ok()).collect()).filter(|v: &Vec<f32>| !v.is_empty()).unwrap_or_else(|| vec![0.25, 0.5, 0.97]);
    let only: Vec<&str> = args.get(7).map(|s| s.split(',').map(str::trim).collect()).unwrap_or_default();
    std::fs::create_dir_all(&dir).unwrap();
    let now = WallTime { year: 2026, month: 11, day: 9, weekday: 0, hour: 21, minute: 47, second: 10 };
    for def in EFFECTS.iter() {
        let id = def.id_str();
        if !only.is_empty() && !only.contains(&id) {
            continue;
        }
        let ini = format!("theme={theme}\neffects={id}\nlang=es\n{extra}\n");
        let cfg = Config::parse(&ini, Rgb::hex(0x0078D4), 7, true);
        let mut e = Engine::new(w, h, cfg);
        let l = e.layout();
        let times: Vec<f32> = fracs.iter().map(|f| 0.15 + def.secs * f).collect();
        let (bw, bh) = (l.buf_w as usize, l.buf_h as usize);
        let mut sheet = Vec::with_capacity(bw * bh * times.len());
        let mut t = 0.0f64;
        for &target in &times {
            while t < target as f64 {
                t += 1.0 / 60.0;
                e.tick(t, now);
            }
            sheet.extend_from_slice(e.pixels());
        }
        let path = format!("{dir}/{id}.png");
        std::fs::write(&path, png::encode_rgb32(bw, bh * times.len(), &sheet)).unwrap();
        println!("{path} {}x{} celdas {}x{}", bw, bh, l.cols, l.rows);
    }
}
