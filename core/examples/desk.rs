//! Renderiza el escritorio entero (varios monitores a escala, con el marco de cada pantalla) en
//! varios momentos de cada efecto de escritorio, para ver cómo pasa de un monitor a otro.
//! cargo run -p ssfx-core --example desk --release -- [carpeta] [efectos] [fotogramas]

#[path = "png.rs"]
mod png;

use ssfx_core::color::Rgb;
use ssfx_core::desk::Screen;
use ssfx_core::fx::{fx_index, EFFECTS};
use ssfx_core::{Config, Engine, WallTime};

/// Escala del escritorio en la imagen.
const K: f64 = 0.25;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = args.get(1).cloned().unwrap_or_else(|| "snaps".into());
    let ids = args.get(2).cloned().unwrap_or_else(|| "relay,comet,tide".into());
    let frames: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(8);
    std::fs::create_dir_all(&dir).unwrap();
    let now = WallTime { year: 2026, month: 11, day: 10, weekday: 1, hour: 12, minute: 0, second: 0 };
    // 1440p, 1080p principal y un vertical a la derecha, a distintas alturas.
    let screens = [
        Screen { x: -2560.0, y: -180.0, w: 2560.0, h: 1440.0, px_w: 2560, px_h: 1440 },
        Screen::px(1920, 1080),
        Screen { x: 1990.0, y: -400.0, w: 1080.0, h: 1920.0, px_w: 1080, px_h: 1920 },
    ];
    let (x0, y0) = screens.iter().fold((f64::MAX, f64::MAX), |a, s| (a.0.min(s.x), a.1.min(s.y)));
    let (x1, y1) = screens.iter().fold((f64::MIN, f64::MIN), |a, s| (a.0.max(s.x + s.w), a.1.max(s.y + s.h)));
    let (cw, ch) = (((x1 - x0) * K) as usize + 1, ((y1 - y0) * K) as usize + 1);
    for id in ids.split(',') {
        let def = &EFFECTS[fx_index(id).expect("efecto")];
        let mut es: Vec<Engine> = (0..screens.len()).map(|me| Engine::with_desk(&screens, me, Config::parse(&format!("effects={id}\ntheme=tahoe\n"), Rgb::hex(0x0078D4), 5, true))).collect();
        let mut sheet = vec![0x0010_1014u32; cw * ch * frames];
        let mut t = 0.0f64;
        for f in 0..frames {
            let target = 0.15 + def.secs as f64 * (f as f64 + 0.5) / frames as f64;
            while t < target {
                t += 1.0 / 60.0;
                for e in es.iter_mut() {
                    e.tick(t, now);
                }
            }
            let canvas = &mut sheet[f * cw * ch..(f + 1) * cw * ch];
            for (s, e) in screens.iter().zip(&es) {
                let l = e.layout();
                let px = e.pixels();
                let (sx, sy) = (((s.x - x0) * K) as usize, ((s.y - y0) * K) as usize);
                let (sw, sh) = ((s.w * K) as usize, (s.h * K) as usize);
                for y in 0..sh {
                    for x in 0..sw {
                        // Píxel de pantalla -> píxel del búfer (vecino más cercano).
                        let (px_x, px_y) = (x as f64 / K, y as f64 / K);
                        let bx = ((px_x - l.dst_x as f64) * l.buf_w as f64 / l.dst_w as f64).floor() as i64;
                        let by = ((px_y - l.dst_y as f64) * l.buf_h as f64 / l.dst_h as f64).floor() as i64;
                        let edge = x == 0 || y == 0 || x == sw - 1 || y == sh - 1;
                        let c = if edge {
                            0x0050_5060
                        } else if bx >= 0 && by >= 0 && (bx as i32) < l.buf_w && (by as i32) < l.buf_h {
                            px[by as usize * l.buf_w as usize + bx as usize]
                        } else {
                            0
                        };
                        canvas[(sy + y) * cw + sx + x] = c;
                    }
                }
            }
        }
        let path = format!("{dir}/desk-{id}.png");
        std::fs::write(&path, png::encode_rgb32(cw, ch * frames, &sheet)).unwrap();
        println!("{path} {cw}x{}", ch * frames);
    }
}
