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

#[cfg(test)]
mod tests {
    use super::*;

    fn dist(a: Rgb, b: Rgb) -> i32 {
        (a.0 as i32 - b.0 as i32).abs() + (a.1 as i32 - b.1 as i32).abs() + (a.2 as i32 - b.2 as i32).abs()
    }

    /// Lo que garantiza `build_palette`: 2..=4 colores legibles y distintos, o el respaldo.
    fn assert_valid_palette(p: &[Rgb], what: &str) {
        assert!((2..=4).contains(&p.len()), "{what}: {} colores", p.len());
        if p == FALLBACK {
            return;
        }
        for (i, &c) in p.iter().enumerate() {
            assert!(c.u32() <= 0x00FF_FFFF, "{what}");
            assert!(c != BLACK, "{what}: negro invisible sobre el fondo");
            assert!(luminance(c) >= 0.22, "{what}: {c:?} demasiado oscuro ({})", luminance(c));
            for &o in &p[..i] {
                assert!(dist(c, o) > 60, "{what}: {c:?} y {o:?} casi iguales");
            }
        }
    }

    #[test]
    fn hex_and_u32_round_trip() {
        assert_eq!(Rgb::hex(0x123456), Rgb(0x12, 0x34, 0x56));
        assert_eq!(Rgb(0x12, 0x34, 0x56).u32(), 0x0012_3456);
        assert_eq!(WHITE.u32(), 0x00FF_FFFF);
        assert_eq!(BLACK.u32(), 0);
        // El byte alto se ignora (0x00RRGGBB).
        assert_eq!(Rgb::hex(0xFF12_3456), Rgb(0x12, 0x34, 0x56));
        for v in [0u32, 1, 0xFF, 0xFF00, 0xFF0000, 0x7F7F7F, 0xABCDEF] {
            assert_eq!(Rgb::hex(v).u32(), v);
        }
    }

    #[test]
    fn mix_endpoints_and_clamp() {
        let (a, b) = (Rgb(10, 20, 30), Rgb(210, 120, 0));
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
        assert_eq!(mix(a, b, -5.0), a);
        assert_eq!(mix(a, b, 7.0), b);
        assert_eq!(mix(a, b, 0.5), Rgb(110, 70, 15));
        assert_eq!(mix(a, b, f32::NAN), mix(a, b, f32::NAN)); // no entra en pánico
    }

    #[test]
    fn scale_saturates() {
        assert_eq!(scale(Rgb(100, 200, 50), 0.0), BLACK);
        assert_eq!(scale(Rgb(100, 200, 50), -3.0), BLACK);
        assert_eq!(scale(Rgb(100, 200, 50), 1.0), Rgb(100, 200, 50));
        assert_eq!(scale(Rgb(100, 200, 50), 2.0), Rgb(200, 255, 100));
        assert_eq!(scale(WHITE, 100.0), WHITE);
    }

    #[test]
    fn luminance_range() {
        assert_eq!(luminance(BLACK), 0.0);
        assert!((luminance(WHITE) - 1.0).abs() < 1e-4);
        assert!(luminance(Rgb(0, 255, 0)) > luminance(Rgb(255, 0, 0)));
        assert!(luminance(Rgb(255, 0, 0)) > luminance(Rgb(0, 0, 255)));
    }

    #[test]
    fn gradient_is_cyclic_and_safe() {
        assert_eq!(gradient(&[], 0.3), WHITE);
        let pal = [Rgb(255, 0, 0), Rgb(0, 0, 255)];
        assert_eq!(gradient(&pal, 0.0), pal[0]);
        assert_eq!(gradient(&pal, 0.5), pal[1]);
        assert_eq!(gradient(&pal, 1.0), pal[0]);
        assert_eq!(gradient(&pal, -1.0), pal[0]);
        assert_eq!(gradient(&pal, 3.25), gradient(&pal, 0.25));
        assert_eq!(gradient(&[Rgb(1, 2, 3)], 0.77), Rgb(1, 2, 3));
        for t in [-1e6f32, -0.999, 0.999_999, 1e6, f32::MAX, f32::NAN, f32::INFINITY] {
            gradient(&pal, t); // sin pánico
        }
    }

    #[test]
    fn build_palette_rules() {
        assert_eq!(build_palette(&[]), FALLBACK.to_vec());
        assert_eq!(build_palette(&[Rgb(200, 50, 50)]), FALLBACK.to_vec(), "un solo color no es paleta");
        assert_eq!(build_palette(&[WHITE, WHITE, WHITE]), FALLBACK.to_vec(), "sin repetidos");
        // Como mucho cuatro.
        let many: Vec<Rgb> = (0..12).map(|i| Rgb((i * 20) as u8, 255 - (i * 20) as u8, if i % 2 == 0 { 255 } else { 0 })).collect();
        let p = build_palette(&many);
        assert_eq!(p.len(), 4);
        assert_valid_palette(&p, "many");
        // Los oscuros se aclaran hasta leerse sobre negro.
        let p = build_palette(&[BLACK, Rgb(0, 0, 80), Rgb(80, 0, 0)]);
        assert_valid_palette(&p, "oscuros");
        // Colores ya legibles no se tocan.
        let bright = [Rgb(0x60, 0xCD, 0xFF), Rgb(0xFF, 0x8A, 0x3D)];
        assert_eq!(build_palette(&bright), bright.to_vec());
    }

    #[test]
    fn every_theme_is_valid() {
        let mut seen = Vec::new();
        for t in THEMES.iter() {
            assert!(!t.id_str().is_empty() && !t.name_str().is_empty());
            assert_eq!(t.id_str(), t.id_str().to_ascii_lowercase(), "ids en minúsculas");
            assert!(!t.id_str().contains(char::is_whitespace));
            assert!(!seen.contains(&t.id_str()), "id repetido {}", t.id_str());
            seen.push(t.id_str());
            assert!(t.colors.iter().all(|&c| c <= 0x00FF_FFFF), "{}", t.id_str());
            let p = t.palette();
            assert_valid_palette(&p, t.id_str());
            assert_ne!(p, FALLBACK.to_vec(), "el tema {} acaba en la paleta de respaldo", t.id_str());
        }
    }

    #[test]
    fn theme_index_lookup() {
        for (i, t) in THEMES.iter().enumerate() {
            assert_eq!(theme_index(t.id_str()), Some(i));
            assert_eq!(theme_index(&t.id_str().to_ascii_uppercase()), Some(i));
        }
        assert_eq!(theme_index(""), None);
        assert_eq!(theme_index("auto"), None);
        assert_eq!(theme_index("random"), None);
        assert_eq!(theme_index(" aero"), None);
    }

    #[test]
    fn hsl_round_trip() {
        for v in [0xFF0000u32, 0x00FF00, 0x0000FF, 0x0078D4, 0x8E44AD, 0x3DDC97, 0xFF8A3D, 0x808080, 0xFFFFFF, 0] {
            let c = Rgb::hex(v);
            let (h, s, l) = to_hsl(c);
            assert!((0.0..360.0).contains(&h) && (0.0..=1.0).contains(&s) && (0.0..=1.0).contains(&l), "{v:06x}");
            let back = from_hsl(h, s, l);
            assert!(dist(c, back) <= 3, "{v:06x} -> {back:?}");
        }
    }

    #[test]
    fn accent_palette_uses_the_accent() {
        // Acento ya legible: es el primer color tal cual.
        for v in [0x60CDFFu32, 0xFF8A3D, 0x3DDC97, 0xFF7EB6, 0xE81123] {
            let accent = Rgb::hex(v);
            if luminance(accent) < 0.22 {
                continue;
            }
            let p = accent_palette(accent);
            assert_eq!(p[0], accent, "{v:06x}");
            assert_valid_palette(&p, &format!("{v:06x}"));
        }
        // Acento oscuro (azul de Windows): primer color = acento aclarado, sigue siendo azul.
        let p = accent_palette(Rgb::hex(0x0078D4));
        assert_valid_palette(&p, "0078d4");
        assert!(p[0].2 > p[0].0 && p[0].2 >= p[0].1, "{:?} ya no es azul", p[0]);
        // Acentos distintos dan paletas distintas.
        assert_ne!(accent_palette(Rgb::hex(0xE81123)), accent_palette(Rgb::hex(0x10893E)));
    }

    #[test]
    fn accent_palette_special_cases() {
        // Negro: Tokyo Night.
        let tokyo = [0x7AA2F7, 0xBB9AF7, 0x7DCFFF, 0x9ECE6A].map(Rgb::hex);
        for a in [BLACK, Rgb(10, 10, 12)] {
            let p = accent_palette(a);
            assert_valid_palette(&p, "negro");
            assert_eq!(p[0], tokyo[0]);
            assert!(p.iter().all(|c| tokyo.contains(c)), "{p:?}");
        }
        // Gris (Graphite): grises casi neutros.
        let p = accent_palette(Rgb(0x8C, 0x8C, 0x8C));
        assert_valid_palette(&p, "gris");
        assert!(p.iter().all(|c| (c.0 as i32 - c.2 as i32).abs() < 40), "{p:?}");
        // Blanco puro no rompe nada.
        assert_valid_palette(&accent_palette(WHITE), "blanco");
    }

    #[test]
    fn accent_palette_sweep_is_always_valid() {
        for r in (0..=255).step_by(17) {
            for g in (0..=255).step_by(51) {
                for b in (0..=255).step_by(85) {
                    let a = Rgb(r as u8, g as u8, b as u8);
                    assert_valid_palette(&accent_palette(a), &format!("{a:?}"));
                }
            }
        }
    }
}
