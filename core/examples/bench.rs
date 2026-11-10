//! Coste por cuadro de cada efecto (motor + rasterizado, sin presentar en pantalla).
//! cargo run -p ssfx-core --example bench --release -- [presupuesto_ms]
//!
//! Recorre cada efecto entero a 60 fps en 1080p, 1440p y 4K con el tamaño por defecto e imprime
//! p50/p99/máximo del `tick` y la fracción media del búfer que cambia (lo que luego se copia a la
//! pantalla). Con `presupuesto_ms` sale con error si el p99 de algún efecto lo supera.

use ssfx_core::color::Rgb;
use ssfx_core::fx::EFFECTS;
use ssfx_core::{Config, Engine, WallTime};
use std::time::Instant;

const RES: [(i32, i32, &str); 7] = [
    (800, 600, "SVGA"),
    (1366, 768, "768p"),
    (1920, 1080, "1080p"),
    (2560, 1440, "1440p"),
    (3440, 1440, "UW"),
    (3840, 2160, "4K"),
    (7680, 4320, "8K"),
];

fn pct(sorted: &[f64], p: f64) -> f64 {
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

fn main() {
    let budget: Option<f64> = std::env::args().nth(1).and_then(|s| s.parse().ok());
    let now = WallTime { year: 2026, month: 11, day: 9, weekday: 0, hour: 21, minute: 47, second: 10 };
    let mut worst = (0.0f64, String::new());
    println!("{:<12} {:>6} {:>10} {:>8} {:>8} {:>8} {:>8}", "efecto", "res", "búfer", "p50 ms", "p99 ms", "máx ms", "sucio %");
    for &(w, h, name) in &RES {
        for def in EFFECTS.iter() {
            let id = def.id_str();
            let cfg = Config::parse(&format!("effects={id}\nlang=es\n"), Rgb::hex(0x0078D4), 7, true);
            let mut e = Engine::new(w, h, cfg);
            let l = e.layout();
            let area = (l.buf_w as f64) * (l.buf_h as f64);
            // Calentamiento (primer cuadro, asignaciones) fuera de la medida.
            e.tick(0.0, now);
            let frames = ((0.15 + def.secs as f64) * 60.0).ceil() as usize + 30;
            let mut times = Vec::with_capacity(frames);
            let mut dirty = 0.0f64;
            for i in 1..=frames {
                let t = i as f64 / 60.0;
                let s = Instant::now();
                e.tick(t, now);
                times.push(s.elapsed().as_secs_f64() * 1000.0);
                if let Some((_, _, dw, dh)) = e.dirty() {
                    dirty += dw as f64 * dh as f64 / area;
                }
            }
            times.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let p99 = pct(&times, 0.99);
            if p99 > worst.0 {
                worst = (p99, format!("{id} a {name}"));
            }
            println!(
                "{:<12} {:>6} {:>10} {:>8.3} {:>8.3} {:>8.3} {:>8.1}",
                id,
                name,
                format!("{}x{}", l.buf_w, l.buf_h),
                pct(&times, 0.5),
                p99,
                times[times.len() - 1],
                dirty / frames as f64 * 100.0
            );
        }
    }
    println!("Peor p99: {:.3} ms ({})", worst.0, worst.1);
    if let Some(b) = budget {
        if worst.0 > b {
            eprintln!("Supera el presupuesto de {b} ms por cuadro");
            std::process::exit(1);
        }
        println!("Dentro del presupuesto de {b} ms por cuadro");
    }
}
