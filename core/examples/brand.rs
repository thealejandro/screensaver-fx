//! Genera el icono y las imágenes del instalador con el propio motor.
//! cargo run -p ssfx-core --example brand --release -- installer/assets

#[path = "png.rs"]
mod png;

use ssfx_core::art::Art;
use ssfx_core::color::{gradient, Rgb};
use ssfx_core::fx::settle;
use ssfx_core::grid::Grid;
use ssfx_core::raster::Raster;
use ssfx_core::rng::Rng;

/// Tokyo Night: el tema por defecto de Omarchy.
const PAL: [u32; 4] = [0x7AA2F7, 0xBB9AF7, 0x7DCFFF, 0x9ECE6A];
const BG_TOP: [f32; 3] = [13.0, 16.0, 23.0];
const BG_BOT: [f32; 3] = [22.0, 26.0, 43.0];
const FG: [f32; 3] = [192.0, 202.0, 245.0];

fn pal() -> Vec<Rgb> {
    PAL.map(Rgb::hex).to_vec()
}

fn rgbf(c: Rgb) -> [f32; 3] {
    [c.0 as f32, c.1 as f32, c.2 as f32]
}

struct Canvas {
    w: usize,
    h: usize,
    px: Vec<[f32; 4]>,
}

impl Canvas {
    fn new(w: usize, h: usize) -> Canvas {
        Canvas { w, h, px: vec![[0.0; 4]; w * h] }
    }

    fn blend(&mut self, x: i32, y: i32, c: [f32; 3], a: f32) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h || a <= 0.0 {
            return;
        }
        let p = &mut self.px[y as usize * self.w + x as usize];
        let a = a.min(1.0);
        for i in 0..3 {
            p[i] = p[i] * (1.0 - a) + c[i] * a;
        }
        p[3] = p[3] + a * (1.0 - p[3]);
    }

    fn background(&mut self, seed: u64) {
        self.starfield(seed, 450, (self.w / 160).max(1) as i32);
    }

    /// Degradado de fondo con una estrella por cada `per` píxeles, de `dot` píxeles de lado.
    fn starfield(&mut self, seed: u64, per: usize, dot: i32) {
        for y in 0..self.h {
            let t = y as f32 / self.h.max(1) as f32;
            let c = [0, 1, 2].map(|i| BG_TOP[i] + (BG_BOT[i] - BG_TOP[i]) * t);
            for x in 0..self.w {
                self.px[y * self.w + x] = [c[0], c[1], c[2], 1.0];
            }
        }
        let mut r = Rng::new(seed);
        let n = self.w * self.h / per;
        for _ in 0..n {
            let (x, y) = (r.below(self.w) as i32, r.below(self.h) as i32);
            let b = r.range(0.15, 0.7);
            for dy in 0..dot {
                for dx in 0..dot {
                    self.blend(x + dx, y + dy, [230.0, 235.0, 255.0], b);
                }
            }
        }
    }

    /// Pega un búfer 0x00RRGGBB (negro = transparente) reducido por promedio de área.
    fn paste(&mut self, src: &[u32], sw: usize, sh: usize, x0: f32, y0: f32, dw: f32) {
        let s = sw as f32 / dw;
        let dh = sh as f32 / s;
        for dy in 0..dh.ceil() as i32 {
            for dx in 0..dw.ceil() as i32 {
                let (ax, ay) = ((dx as f32 * s) as usize, (dy as f32 * s) as usize);
                let (bx, by) = ((((dx + 1) as f32 * s) as usize).min(sw).max(ax + 1), (((dy + 1) as f32 * s) as usize).min(sh).max(ay + 1));
                let (mut acc, mut on, mut total) = ([0.0f32; 3], 0.0f32, 0.0f32);
                for yy in ay..by.min(sh) {
                    for xx in ax..bx.min(sw) {
                        total += 1.0;
                        let p = src[yy * sw + xx];
                        if p != 0 {
                            on += 1.0;
                            acc[0] += (p >> 16 & 0xFF) as f32;
                            acc[1] += (p >> 8 & 0xFF) as f32;
                            acc[2] += (p & 0xFF) as f32;
                        }
                    }
                }
                if on > 0.0 {
                    self.blend(x0 as i32 + dx, y0 as i32 + dy, acc.map(|v| v / on), on / total);
                }
            }
        }
    }

    /// Texto con font8x8 a escala entera.
    fn text(&mut self, s: &str, cx: f32, y: i32, k: i32, c: [f32; 3]) {
        use font8x8::UnicodeFonts;
        let w = s.chars().count() as i32 * 8 * k;
        let mut x = cx as i32 - w / 2;
        for ch in s.chars() {
            let g = font8x8::BASIC_FONTS.get(ch).unwrap_or([0; 8]);
            for (r, row) in g.iter().enumerate() {
                for b in 0..8 {
                    if row >> b & 1 == 1 {
                        for dy in 0..k {
                            for dx in 0..k {
                                self.blend(x + b * k + dx, y + r as i32 * k + dy, c, 1.0);
                            }
                        }
                    }
                }
            }
            x += 8 * k;
        }
    }

    fn png(&self, alpha: bool) -> Vec<u8> {
        let ch = if alpha { 4 } else { 3 };
        let mut data = Vec::with_capacity(self.w * self.h * ch);
        for p in &self.px {
            data.extend_from_slice(&[p[0], p[1], p[2]].map(|v| v.round().clamp(0.0, 255.0) as u8));
            if alpha {
                data.push((p[3] * 255.0).round().clamp(0.0, 255.0) as u8);
            }
        }
        png::encode(self.w, self.h, ch, &data)
    }
}

/// Rótulo por defecto rasterizado a 8x16 por celda, con su degradado final.
fn logo() -> (Vec<u32>, usize, usize) {
    let art = Art::default_art();
    let p = pal();
    let mut g = Grid::new(art.w, art.h);
    for &(x, y, ch) in &art.cells {
        g.put(x, y, ch, settle(&p, &art, x, y, 0.0));
    }
    let mut r = Raster::new(art.w, art.h);
    r.update(&g);
    (r.px.clone(), r.w, r.h)
}

/// Imagen lateral de las páginas de bienvenida y final (proporción 164x314).
fn wizard(w: usize, h: usize) -> Canvas {
    let mut c = Canvas::new(w, h);
    c.background(11);
    let (px, lw, lh) = logo();
    let dw = w as f32 * 0.8;
    let dh = lh as f32 * dw / lw as f32;
    c.paste(&px, lw, lh, (w as f32 - dw) / 2.0, h as f32 * 0.36 - dh / 2.0, dw);
    let k = (w / 164).max(1) as i32;
    c.text("SCREENSAVER", w as f32 / 2.0, (h as f32 * 0.6) as i32, k, FG);
    c.text("FX", w as f32 / 2.0, (h as f32 * 0.6) as i32 + 11 * k, k, rgbf(gradient(&pal(), 0.3)));
    c
}

fn sd_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (pa, ba) = ((p.0 - a.0, p.1 - a.1), (b.0 - a.0, b.1 - a.1));
    let h = ((pa.0 * ba.0 + pa.1 * ba.1) / (ba.0 * ba.0 + ba.1 * ba.1)).clamp(0.0, 1.0);
    (pa.0 - ba.0 * h).hypot(pa.1 - ba.1 * h)
}

fn sd_round_rect(p: (f32, f32), half: f32, r: f32) -> f32 {
    let q = ((p.0.abs() - half + r).max(0.0), (p.1.abs() - half + r).max(0.0));
    q.0.hypot(q.1) + (p.0.abs() - half + r).max(p.1.abs() - half + r).min(0.0) - r
}

/// Icono: cuadrado redondeado oscuro con un prompt «❯ _» en degradado.
fn icon(s: usize, transparent: bool) -> Canvas {
    let mut c = Canvas::new(s, s);
    let p = pal();
    let sf = s as f32;
    let px = 1.0 / sf;
    for y in 0..s {
        for x in 0..s {
            // Coordenadas normalizadas 0..1 con el centro del píxel.
            let q = ((x as f32 + 0.5) / sf, (y as f32 + 0.5) / sf);
            let d = sd_round_rect((q.0 - 0.5, q.1 - 0.5), 0.47, 0.2);
            let a = (0.5 - d / px).clamp(0.0, 1.0);
            if a <= 0.0 && transparent {
                continue;
            }
            let t = q.1;
            let bg = [0, 1, 2].map(|i| BG_TOP[i] + (BG_BOT[i] - BG_TOP[i]) * t);
            c.blend(x as i32, y as i32, bg, if transparent { a } else { 1.0 });
            let stroke = 0.075;
            let chev = sd_segment(q, (0.25, 0.3), (0.47, 0.5)).min(sd_segment(q, (0.47, 0.5), (0.25, 0.7)));
            let bar = sd_segment(q, (0.56, 0.7), (0.76, 0.7));
            let ca = (0.5 - (chev - stroke) / px).clamp(0.0, 1.0) * a;
            if ca > 0.0 {
                c.blend(x as i32, y as i32, rgbf(gradient(&p, q.1 * 0.5)), ca);
            }
            let ba = (0.5 - (bar - stroke) / px).clamp(0.0, 1.0) * a;
            if ba > 0.0 {
                c.blend(x as i32, y as i32, rgbf(gradient(&p, 0.55 + q.0 * 0.3)), ba);
            }
        }
    }
    c
}

/// ICO con entradas PNG (Windows Vista o posterior).
fn ico(sizes: &[usize]) -> Vec<u8> {
    let imgs: Vec<Vec<u8>> = sizes.iter().map(|&s| icon(s, true).png(true)).collect();
    let mut out = vec![0, 0, 1, 0];
    out.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len();
    for (&s, img) in sizes.iter().zip(&imgs) {
        let b = if s >= 256 { 0 } else { s as u8 };
        out.extend_from_slice(&[b, b, 0, 0, 1, 0, 32, 0]);
        out.extend_from_slice(&(img.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += img.len();
    }
    for img in imgs {
        out.extend_from_slice(&img);
    }
    out
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "installer/assets".into());
    std::fs::create_dir_all(&dir).unwrap();
    let save = |name: &str, data: Vec<u8>| {
        std::fs::write(format!("{dir}/{name}"), data).unwrap();
        println!("{dir}/{name}");
    };
    save("icon.ico", ico(&[16, 20, 24, 32, 40, 48, 64, 128, 256]));
    save("icon-512.png", icon(512, true).png(true));
    // Asistente de Windows (100 %, 150 %, 200 %).
    for (w, h) in [(164, 314), (246, 459), (328, 628)] {
        save(&format!("wizard-{w}.png"), wizard(w, h).png(false));
    }
    for s in [55, 83, 110] {
        save(&format!("small-{s}.png"), icon(s, true).png(true));
    }
    // macOS: fondo del instalador .pkg y miniaturas de Ajustes del Sistema.
    save("pkg-background.png", wizard(328, 628).png(false));
    save("pkg-mark.png", icon(128, true).png(true));
    save("thumbnail.png", thumbnail(90, 58).png(false));
    save("thumbnail@2x.png", thumbnail(180, 116).png(false));
    // macOS: fondo de la ventana del .dmg (macos/package.sh las une en un TIFF para Retina).
    save("dmg-background.png", dmg(1).png(false));
    save("dmg-background@2x.png", dmg(2).png(false));
}

/// Posiciones de los iconos en la ventana del .dmg (puntos, a 1x); macos/package.sh usa las mismas.
const DMG_ICONS: [f32; 2] = [170.0, 470.0];
const DMG_ICON_Y: f32 = 235.0;

/// Fondo del .dmg (640x400 puntos, escala `k`): rótulo, flecha del .saver a «Screen Savers» y,
/// bajo cada icono, una pastilla de tono medio donde Finder escribe el nombre, legible tanto con
/// la letra negra del modo claro como con la blanca del modo oscuro (contraste ~4,5:1 con ambas).
fn dmg(k: usize) -> Canvas {
    let (w, h, kf) = (640 * k, 400 * k, k as f32);
    let mut c = Canvas::new(w, h);
    c.starfield(23, 2400 * k * k, k as i32);
    let (px, lw, lh) = logo();
    let dw = (300.0f32).min(100.0 * lw as f32 / lh as f32) * kf;
    let dh = lh as f32 * dw / lw as f32;
    c.paste(&px, lw, lh, (w as f32 - dw) / 2.0, 95.0 * kf - dh / 2.0, dw);
    let p = pal();
    let (ax, bx, y0) = (DMG_ICONS[0] + 90.0, DMG_ICONS[1] - 90.0, DMG_ICON_Y);
    let pill = [106.0, 113.0, 150.0];
    for y in 0..h {
        for x in 0..w {
            // Coordenadas en puntos (1x) con el centro del píxel.
            let q = ((x as f32 + 0.5) / kf, (y as f32 + 0.5) / kf);
            let d = sd_segment(q, (ax, y0), (bx, y0)).min(sd_segment(q, (bx - 16.0, y0 - 15.0), (bx, y0))).min(sd_segment(q, (bx - 16.0, y0 + 15.0), (bx, y0)));
            let a = (0.5 - (d - 2.5) * kf).clamp(0.0, 1.0);
            if a > 0.0 {
                c.blend(x as i32, y as i32, rgbf(gradient(&p, (q.0 - ax) / (bx - ax) * 0.6)), a);
            }
            for cx in DMG_ICONS {
                let (dx, dy) = ((q.0 - cx).abs() - 74.0, (q.1 - 304.0).abs());
                let r = 12.0;
                let sd = (dx.max(0.0)).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - r;
                let a = (0.5 - sd * kf).clamp(0.0, 1.0);
                if a > 0.0 {
                    c.blend(x as i32, y as i32, pill, a * 0.92);
                }
            }
        }
    }
    let half = w as f32 / 2.0;
    c.text("Arrastra a Screen Savers", half, (342.0 * kf) as i32, 2 * k as i32, FG);
    c.text("Drag onto Screen Savers to install", half, (368.0 * kf) as i32, k as i32, rgbf(gradient(&p, 0.3)));
    c
}

/// Miniatura horizontal: el rótulo sobre el fondo estrellado.
fn thumbnail(w: usize, h: usize) -> Canvas {
    let mut c = Canvas::new(w, h);
    c.background(5);
    let (px, lw, lh) = logo();
    let dw = w as f32 * 0.72;
    let dh = lh as f32 * dw / lw as f32;
    c.paste(&px, lw, lh, (w as f32 - dw) / 2.0, (h as f32 - dh) / 2.0, dw);
    c
}
