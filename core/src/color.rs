//! Colores: mezcla, luminancia, paleta del tema (misma lógica que screensaver-fx.py) y temas.

use core::ffi::CStr;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rgb(pub u8, pub u8, pub u8);

pub const WHITE: Rgb = Rgb(255, 255, 255);
pub const BLACK: Rgb = Rgb(0, 0, 0);

impl Rgb {
    pub const fn hex(v: u32) -> Rgb {
        Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
    }
    /// 0x00RRGGBB: en memoria little-endian queda B,G,R,X (DIB de Windows y CGImage BGRX).
    pub const fn u32(self) -> u32 {
        (self.0 as u32) << 16 | (self.1 as u32) << 8 | self.2 as u32
    }
}

fn r8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

pub fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| r8(x as f32 + (y as f32 - x as f32) * t);
    Rgb(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

pub fn scale(c: Rgb, k: f32) -> Rgb {
    Rgb(r8(c.0 as f32 * k), r8(c.1 as f32 * k), r8(c.2 as f32 * k))
}

pub fn luminance(c: Rgb) -> f32 {
    let lin = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.0) + 0.7152 * lin(c.1) + 0.0722 * lin(c.2)
}

/// Gradiente cíclico sobre la paleta, t en cualquier rango.
pub fn gradient(pal: &[Rgb], t: f32) -> Rgb {
    let n = pal.len();
    if n == 0 {
        return WHITE;
    }
    let t = t.rem_euclid(1.0) * n as f32;
    let i = t as usize;
    mix(pal[i % n], pal[(i + 1) % n], t - i as f32)
}

pub const FALLBACK: [Rgb; 2] = [Rgb::hex(0x8A008A), Rgb::hex(0x00D1FF)];

/// Colores aclarados lo justo para leerse sobre negro, sin repetidos, máximo 4.
pub fn build_palette(colors: &[Rgb]) -> Vec<Rgb> {
    let mut out: Vec<Rgb> = Vec::with_capacity(4);
    for &orig in colors {
        let (mut c, mut t) = (orig, 0.0f32);
        while luminance(c) < 0.22 && t < 1.0 {
            t += 0.05;
            c = mix(orig, WHITE, t);
        }
        let far = |o: &Rgb| {
            (c.0 as i32 - o.0 as i32).abs() + (c.1 as i32 - o.1 as i32).abs() + (c.2 as i32 - o.2 as i32).abs() > 60
        };
        if out.iter().all(far) {
            out.push(c);
        }
        if out.len() == 4 {
            break;
        }
    }
    if out.len() < 2 {
        out = FALLBACK.to_vec();
    }
    out
}

fn to_hsl(c: Rgb) -> (f32, f32, f32) {
    let (r, g, b) = (c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0);
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let l = (mx + mn) / 2.0;
    if mx == mn {
        return (0.0, 0.0, l);
    }
    let d = mx - mn;
    let s = if l > 0.5 { d / (2.0 - mx - mn) } else { d / (mx + mn) };
    let h = if mx == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if mx == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

fn from_hsl(h: f32, s: f32, l: f32) -> Rgb {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    Rgb(r8((r + m) * 255.0), r8((g + m) * 255.0), r8((b + m) * 255.0))
}

/// Paleta a partir del color de acento del sistema: el acento y tonos vecinos.
pub fn accent_palette(accent: Rgb) -> Vec<Rgb> {
    let (h, s, l) = to_hsl(accent);
    let cands: Vec<Rgb> = if s < 0.12 && l < 0.12 {
        // Acento negro (sin color elegido): Tokyo Night, el tema por defecto de Omarchy.
        [0x7AA2F7, 0xBB9AF7, 0x7DCFFF, 0x9ECE6A].map(Rgb::hex).to_vec()
    } else if s < 0.12 {
        // Acento gris (Graphite en macOS): escala de grises con un toque frío.
        vec![accent, mix(accent, WHITE, 0.45), Rgb(0x8E, 0x9A, 0xAF), mix(accent, WHITE, 0.8)]
    } else {
        let (s2, l2) = (s.max(0.55), l.clamp(0.5, 0.68));
        vec![
            accent,
            from_hsl(h + 40.0, s2, l2),
            from_hsl(h - 45.0, s2, l2),
            from_hsl(h + 180.0, s2 * 0.9, l2),
            from_hsl(h + 90.0, s2, l2),
        ]
    };
    build_palette(&cands)
}

pub struct ThemeDef {
    pub id: &'static CStr,
    pub name: &'static CStr,
    /// accent, bright_blue, bright_cyan, bright_magenta, magenta, cyan, blue, bright_green
    pub colors: [u32; 8],
}

impl ThemeDef {
    pub fn id_str(&self) -> &'static str {
        self.id.to_str().unwrap_or("")
    }
    pub fn name_str(&self) -> &'static str {
        self.name.to_str().unwrap_or("")
    }
    pub fn palette(&self) -> Vec<Rgb> {
        let cs: Vec<Rgb> = self.colors.iter().map(|&v| Rgb::hex(v)).collect();
        build_palette(&cs)
    }
}

/// Los temas de Omarchy de thealejandro (colors.toml de cada repositorio).
pub static THEMES: [ThemeDef; 11] = [
    ThemeDef { id: c"aero", name: c"Aero", colors: [0x0073b7, 0x0073b7, 0x007a8a, 0x984fb9, 0x8e44ad, 0x007a86, 0x1a6fc0, 0x00811b] },
    ThemeDef { id: c"aero-night", name: c"Aero Night", colors: [0x4fc3f7, 0x8cc4ff, 0x81d4fa, 0xd6bbff, 0xc39bff, 0x4fc3f7, 0x5aa9ff, 0x9fe3a9] },
    ThemeDef { id: c"obsidiana", name: c"Obsidiana", colors: [0x3ddc97, 0x85c0ff, 0x7fe0d7, 0xd8b2f0, 0xc792ea, 0x4fd1c5, 0x5aa9ff, 0x6ee7b4] },
    ThemeDef { id: c"fluent", name: c"Fluent", colors: [0x60cdff, 0x7fd4ff, 0x99ebff, 0xe3bcff, 0xd59dff, 0x60cdff, 0x4cc2ff, 0x94de89] },
    ThemeDef { id: c"fluent-light", name: c"Fluent Light", colors: [0x005fb8, 0x0070cc, 0x007a89, 0x805fad, 0x815eb2, 0x007b88, 0x005fb8, 0x008200] },
    ThemeDef { id: c"tahoe", name: c"Tahoe", colors: [0x0067df, 0x0067df, 0x0072a2, 0x9c3ec9, 0x7d3fb0, 0x00759b, 0x0058d1, 0x007e18] },
    ThemeDef { id: c"ventura", name: c"Ventura", colors: [0xff8a3d, 0x7fadff, 0x88d6e3, 0xeb9fcd, 0xe07ab8, 0x5ec4d6, 0x4f8dff, 0xb8dc8e] },
    ThemeDef { id: c"big-sur", name: c"Big Sur", colors: [0xff7eb6, 0xa3aeff, 0xa3e2f6, 0xe4b0ff, 0xd48cff, 0x7cd4f0, 0x7c8cff, 0xa0ecca] },
    ThemeDef { id: c"graphite", name: c"Graphite", colors: [0xa1a1a6, 0xe4e4e4, 0xacacac, 0xa0a0a0, 0x8c8c8c, 0x989898, 0xd0d0d0, 0xb8b8b8] },
    ThemeDef { id: c"cupertino", name: c"Cupertino", colors: [0x0a84ff, 0x409cff, 0x8adcff, 0xda8fff, 0xbf5af2, 0x64d2ff, 0x0a84ff, 0x5de87a] },
    ThemeDef { id: c"cupertino-light", name: c"Cupertino Light", colors: [0x006aed, 0x006aed, 0x007a91, 0xa143cf, 0x8944ab, 0x0071a4, 0x0040dd, 0x00831e] },
];

pub fn theme_index(id: &str) -> Option<usize> {
    THEMES.iter().position(|t| t.id_str().eq_ignore_ascii_case(id))
}
