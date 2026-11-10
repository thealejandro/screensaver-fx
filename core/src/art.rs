//! Rótulo: el texto que se anima. Por defecto JACC + XACODE (como generate-screensaver.py).

use crate::config::Font;
use crate::font;

pub struct Art {
    pub w: i32,
    pub h: i32,
    pub cells: Vec<(i32, i32, char)>,
}

impl Art {
    pub fn parse(text: &str) -> Option<Art> {
        let text = text.trim_start_matches('\u{feff}').replace("\r\n", "\n").replace('\t', "    ");
        let lines: Vec<&str> = text.trim_end_matches('\n').split('\n').collect();
        let w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as i32;
        let mut cells = Vec::new();
        for (y, l) in lines.iter().enumerate() {
            for (x, ch) in l.chars().enumerate() {
                if !ch.is_whitespace() && !ch.is_control() {
                    cells.push((x as i32, y as i32, ch));
                }
            }
        }
        if cells.is_empty() || w > 400 || lines.len() > 200 {
            return None;
        }
        Some(Art { w, h: lines.len() as i32, cells })
    }

    pub fn default_art() -> Art {
        Art::parse(&default_text()).expect("rótulo por defecto")
    }

    /// Rótulo a partir de texto libre: título en letra grande y subtítulo en pequeña.
    pub fn from_text(title: &str, subtitle: &str) -> Option<Art> {
        Art::from_text_with(title, subtitle, Font::Block)
    }

    pub fn from_text_with(title: &str, subtitle: &str, font: Font) -> Option<Art> {
        Art::parse(&banner_text_with(title, subtitle, font))
    }

    /// JACC + XACODE con el estilo de letra pedido (la de píxel no aplica: es un dibujo a mano).
    pub fn default_art_with(font: Font) -> Art {
        let font = if font == Font::Pixel { Font::Block } else { font };
        Art::from_text_with("JACC", "XACODE", font).expect("rótulo por defecto")
    }
}

/// Glifo 8x8 de font8x8 (bit 0 = columna izquierda).
fn glyph(ch: char) -> [u8; 8] {
    use font8x8::UnicodeFonts;
    font8x8::BASIC_FONTS
        .get(ch)
        .or_else(|| font8x8::LATIN_FONTS.get(ch))
        .or_else(|| font8x8::GREEK_FONTS.get(ch))
        .unwrap_or([0; 8])
}

/// Escribe `s` con píxeles de bloque. `big`: cada píxel es "██" (8 filas de alto);
/// si no, medios bloques ▀▄ (4 filas de alto). Las letras se recortan a su ancho real.
fn bitmap_line(s: &str, big: bool) -> Vec<String> {
    let rows = if big { 8 } else { 4 };
    let mut out = vec![String::new(); rows];
    for (i, ch) in s.chars().enumerate() {
        let g = glyph(ch);
        let used = g.iter().fold(0u8, |a, &r| a | r);
        let (c0, c1) = if used == 0 { (0, 3) } else { (used.trailing_zeros(), 7 - used.leading_zeros()) };
        if i > 0 {
            for line in out.iter_mut() {
                line.push_str(if big { "  " } else { " " });
            }
        }
        for (r, line) in out.iter_mut().enumerate() {
            for c in c0..=c1 {
                let on = |row: usize| g[row] >> c & 1 == 1;
                if big {
                    line.push_str(if on(r) { "██" } else { "  " });
                } else {
                    line.push(match (on(r * 2), on(r * 2 + 1)) {
                        (true, true) => '█',
                        (true, false) => '▀',
                        (false, true) => '▄',
                        _ => ' ',
                    });
                }
            }
        }
    }
    out
}

pub const MAX_TITLE: usize = 16;
pub const MAX_SUBTITLE: usize = 32;

fn width(s: &str) -> usize {
    s.chars().count()
}

/// Rellena con espacios a los lados hasta `w` celdas (centrado).
fn pad_center(s: &str, w: usize) -> String {
    let left = w.saturating_sub(width(s)) / 2;
    let right = w.saturating_sub(width(s) + left);
    format!("{}{s}{}", " ".repeat(left), " ".repeat(right))
}

/// Escribe `s` con las letras de bloque de `font.rs`. `big`: 9 filas (más 2 arriba si el texto
/// lleva tildes o Ñ); si no, 4 filas. Lo que no tiene dibujo propio cae en la letra de píxel.
fn block_line(s: &str, big: bool) -> Vec<String> {
    let base = if big { font::BIG_H } else { font::SMALL_H };
    let chars: Vec<char> = s.chars().collect();
    let marked = |ch: char| big && font::decompose(ch).1.is_some() && font::big(font::decompose(ch).0).is_some();
    let top = if chars.iter().any(|&c| marked(c)) { font::MARK_H } else { 0 };
    let mut out = vec![String::new(); top + base];
    for (i, &ch) in chars.iter().enumerate() {
        let (plain, mark) = font::decompose(ch);
        let drawn: Option<Vec<String>> = if big { font::big(plain).map(|g| g.iter().map(|r| r.to_string()).collect()) } else { font::small(ch).map(|g| g.iter().map(|r| r.to_string()).collect()) };
        let mark = mark.filter(|_| big && drawn.is_some());
        let mut rows = drawn.unwrap_or_else(|| {
            let mut px = bitmap_line(&ch.to_string(), big);
            // La letra de píxel grande mide 8 filas: se apoya en la base de las de 9.
            while px.len() < base {
                px.insert(0, " ".repeat(width(&px[0])));
            }
            px
        });
        let mark_rows = mark.map(|m| m.rows());
        let w = width(&rows[0]).max(mark_rows.map_or(0, |m| width(m[0])));
        rows.iter_mut().for_each(|r| *r = pad_center(r, w));
        if i > 0 {
            out.iter_mut().for_each(|line| line.push_str(if big { "   " } else { "  " }));
        }
        for (r, line) in out.iter_mut().enumerate() {
            if r < top {
                line.push_str(&pad_center(mark_rows.map_or("", |m| m[r]), w));
            } else {
                line.push_str(&rows[r - top]);
            }
        }
    }
    out
}

/// Sombra: cada celda dibujada proyecta un ░ una celda abajo a la derecha.
fn add_shadow(lines: &[String]) -> Vec<String> {
    let w = lines.iter().map(|l| width(l)).max().unwrap_or(0) + 1;
    let mut g: Vec<Vec<char>> = (0..=lines.len()).map(|y| lines.get(y).map_or_else(Vec::new, |l| l.chars().collect())).collect();
    g.iter_mut().for_each(|row| row.resize(w, ' '));
    for y in (0..lines.len()).rev() {
        for x in (0..w - 1).rev() {
            if g[y][x] != ' ' && g[y][x] != '░' && g[y + 1][x + 1] == ' ' {
                g[y + 1][x + 1] = '░';
            }
        }
    }
    g.iter().map(|row| row.iter().collect::<String>().trim_end().to_string()).collect()
}

/// Contorno: deja solo el borde de las letras (un píxel de media celda de alto).
fn outline(lines: &[String]) -> Vec<String> {
    let w = lines.iter().map(|l| width(l)).max().unwrap_or(0);
    let h = lines.len() * 2;
    let mut px = vec![false; w * h];
    for (y, l) in lines.iter().enumerate() {
        for (x, ch) in l.chars().enumerate() {
            let (up, down) = match ch {
                '▀' => (true, false),
                '▄' => (false, true),
                ' ' => (false, false),
                _ => (true, true),
            };
            px[2 * y * w + x] = up;
            px[(2 * y + 1) * w + x] = down;
        }
    }
    let on = |x: i32, y: i32| x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && px[y as usize * w + x as usize];
    let edge = |x: usize, y: usize| {
        let (x, y) = (x as i32, y as i32);
        on(x, y) && !(on(x - 1, y) && on(x + 1, y) && on(x, y - 1) && on(x, y + 1) && on(x - 1, y - 1) && on(x + 1, y - 1) && on(x - 1, y + 1) && on(x + 1, y + 1))
    };
    (0..lines.len())
        .map(|y| {
            let row: String = (0..w)
                .map(|x| match (edge(x, 2 * y), edge(x, 2 * y + 1)) {
                    (true, true) => '█',
                    (true, false) => '▀',
                    (false, true) => '▄',
                    _ => ' ',
                })
                .collect();
            row.trim_end().to_string()
        })
        .collect()
}

/// Rótulo de texto con la letra de bloque.
pub fn banner_text(title: &str, subtitle: &str) -> String {
    banner_text_with(title, subtitle, Font::Block)
}

pub fn banner_text_with(title: &str, subtitle: &str, font: Font) -> String {
    let mut title: String = title.trim().chars().take(MAX_TITLE).collect();
    let mut subtitle: String = subtitle.trim().chars().take(MAX_SUBTITLE).collect();
    // Una sola línea (esté en el campo que esté) siempre va en letra grande.
    if title.is_empty() {
        title = subtitle.chars().take(MAX_TITLE).collect();
        subtitle.clear();
    }
    let pixel = font == Font::Pixel;
    let line = |s: &str, big: bool| if pixel { bitmap_line(s, big) } else { block_line(s, big) };
    let mut blocks = Vec::new();
    if !title.is_empty() {
        blocks.push(line(&title, true));
    }
    if !subtitle.is_empty() {
        blocks.push(line(&subtitle, false));
    }
    let total = blocks.iter().map(|b| width(&b[0])).max().unwrap_or(0);
    let mut out: Vec<String> = Vec::new();
    for (i, b) in blocks.iter().enumerate() {
        if i > 0 {
            // La letra de bloque respira mejor con dos filas entre título y subtítulo.
            out.extend(vec![String::new(); if pixel { 1 } else { 2 }]);
        }
        let pad = " ".repeat((total - width(&b[0])) / 2);
        out.extend(b.iter().map(|l| format!("{pad}{l}").trim_end().to_string()));
    }
    // Quita filas vacías arriba y abajo (letras sin trazos en la base, como en minúsculas).
    while out.first().is_some_and(|l| l.is_empty()) {
        out.remove(0);
    }
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    let out = match font {
        Font::Shadow => add_shadow(&out),
        Font::Outline => outline(&out),
        Font::Block | Font::Pixel => out,
    };
    out.join("\n") + "\n"
}

/// El mismo screensaver.txt que genera examples/generate-screensaver.py.
pub fn default_text() -> String {
    banner_text("JACC", "XACODE")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(a: &Art) -> Vec<(i32, i32, char)> {
        a.cells.clone()
    }

    fn assert_consistent(a: &Art) {
        assert!(a.w > 0 && a.h > 0 && a.w <= 400 && a.h <= 200);
        assert!(!a.cells.is_empty());
        for &(x, y, ch) in &a.cells {
            assert!((0..a.w).contains(&x) && (0..a.h).contains(&y), "({x},{y}) fuera de {}x{}", a.w, a.h);
            assert!(!ch.is_whitespace() && !ch.is_control(), "{ch:?}");
        }
        // Sin celdas repetidas.
        let mut pos: Vec<(i32, i32)> = a.cells.iter().map(|&(x, y, _)| (x, y)).collect();
        pos.sort_unstable();
        pos.dedup();
        assert_eq!(pos.len(), a.cells.len());
    }

    #[test]
    fn default_art_shape() {
        let a = Art::default_art();
        assert_consistent(&a);
        // JACC (4 letras de 11 + 3 huecos de 3) sobre XACODE, con dos líneas en blanco.
        assert_eq!((a.w, a.h), (53, 15));
        assert!(a.cells.iter().all(|&(_, _, ch)| "█▀▄".contains(ch)));
        // Filas 9 y 10 vacías (separación).
        assert!(a.cells.iter().all(|&(_, y, _)| y != 9 && y != 10));
        assert!(default_text().ends_with('\n'));
    }

    #[test]
    fn parse_simple_multiline() {
        let a = Art::parse("AB\n C\n").unwrap();
        assert_eq!((a.w, a.h), (2, 2));
        assert_eq!(cells(&a), [(0, 0, 'A'), (1, 0, 'B'), (1, 1, 'C')]);
        assert_consistent(&a);
    }

    #[test]
    fn parse_empty_or_blank_is_none() {
        for s in ["", "\n", "\n\n\n", "   ", " \t \n  \r\n", "\u{feff}", "\u{feff}\n  ", "\u{7}\u{1b}\0"] {
            assert!(Art::parse(s).is_none(), "{s:?}");
        }
    }

    #[test]
    fn parse_bom_and_crlf() {
        let a = Art::parse("\u{feff}X\r\nYY\r\n").unwrap();
        assert_eq!((a.w, a.h), (2, 2));
        assert_eq!(cells(&a), [(0, 0, 'X'), (0, 1, 'Y'), (1, 1, 'Y')]);
        // CR suelto (Mac clásico): es un carácter de control, no ocupa celda propia ni rompe nada.
        let a = Art::parse("A\rB").unwrap();
        assert_consistent(&a);
        assert_eq!(a.h, 1);
    }

    #[test]
    fn parse_tabs_expand_to_four_spaces() {
        let a = Art::parse("\tX\nY\t\tZ").unwrap();
        assert_eq!(cells(&a), [(4, 0, 'X'), (0, 1, 'Y'), (9, 1, 'Z')]);
        assert_eq!(a.w, 10);
    }

    #[test]
    fn parse_counts_chars_not_bytes() {
        let a = Art::parse("ñ█é\n日本").unwrap();
        assert_eq!(a.w, 3);
        assert_eq!(cells(&a), [(0, 0, 'ñ'), (1, 0, '█'), (2, 0, 'é'), (0, 1, '日'), (1, 1, '本')]);
        let a = Art::parse("😀 x").unwrap();
        assert_eq!(cells(&a), [(0, 0, '😀'), (2, 0, 'x')]);
    }

    #[test]
    fn parse_skips_control_and_unicode_spaces() {
        let a = Art::parse("a\u{7}b\u{a0}c\u{3000}d").unwrap();
        assert_eq!(cells(&a), [(0, 0, 'a'), (2, 0, 'b'), (4, 0, 'c'), (6, 0, 'd')]);
    }

    #[test]
    fn parse_keeps_leading_blank_lines_and_trims_trailing() {
        let a = Art::parse("\n\nA\n\n\n").unwrap();
        assert_eq!(a.h, 3);
        assert_eq!(cells(&a), [(0, 2, 'A')]);
    }

    #[test]
    fn parse_size_limits() {
        assert!(Art::parse(&"#".repeat(400)).is_some());
        assert!(Art::parse(&"#".repeat(401)).is_none());
        let tall = |n: usize| vec!["#"; n].join("\n");
        assert!(Art::parse(&tall(200)).is_some());
        assert!(Art::parse(&tall(201)).is_none());
        assert!(Art::parse(&"#".repeat(100_000)).is_none());
        let a = Art::parse(&vec!["#".repeat(400); 200].join("\n")).unwrap();
        assert_eq!((a.w, a.h, a.cells.len()), (400, 200, 80_000));
    }

    #[test]
    fn from_text_title_only_and_subtitle_only_are_the_same() {
        let a = Art::from_text("Hola", "").unwrap();
        let b = Art::from_text("", "Hola").unwrap();
        assert_consistent(&a);
        assert_eq!((a.w, a.h), (b.w, b.h));
        assert_eq!(cells(&a), cells(&b));
        // Letra grande de bloque: 9 filas de bloques y medios bloques.
        assert_eq!(a.h, 9);
        assert!(a.cells.iter().all(|&(_, _, ch)| "█▀▄".contains(ch)));
        // La de píxel sigue como antes: 8 filas como mucho, todas de bloque completo ("██").
        let p = Art::from_text_with("Hola", "", Font::Pixel).unwrap();
        assert!(p.h <= 8);
        assert!(p.cells.iter().all(|&(_, _, ch)| ch == '█'));
    }

    #[test]
    fn block_font_matches_the_hand_drawn_default() {
        assert_eq!(banner_text("JACC", "XACODE"), default_text());
        assert_eq!(banner_text("jacc", "xacode"), default_text());
        assert_eq!(cells(&Art::default_art_with(Font::Pixel)), cells(&Art::default_art()));
    }

    #[test]
    fn accents_add_two_rows_only_when_needed() {
        let plain = Art::from_text("NANDU", "").unwrap();
        let marked = Art::from_text("ÑANDÚ", "").unwrap();
        assert_consistent(&marked);
        assert_eq!((plain.h, marked.h), (9, 11));
        assert_eq!(plain.w, marked.w);
        // La letra en sí no cambia: solo se añade la marca encima.
        let below: Vec<_> = marked.cells.iter().filter(|c| c.1 >= 2).map(|&(x, y, ch)| (x, y - 2, ch)).collect();
        assert_eq!(below, cells(&plain));
        // Una I con tilde se ensancha para que quepa la marca.
        assert!(Art::from_text("Í", "").unwrap().w > Art::from_text("I", "").unwrap().w);
    }

    #[test]
    fn shadow_and_outline_styles() {
        let block = Art::from_text("Hola", "mundo").unwrap();
        let shadow = Art::from_text_with("Hola", "mundo", Font::Shadow).unwrap();
        assert_consistent(&shadow);
        assert_eq!((shadow.w, shadow.h), (block.w + 1, block.h + 1));
        // La sombra solo añade ░ en huecos: las letras quedan intactas.
        let solid: Vec<_> = shadow.cells.iter().copied().filter(|c| c.2 != '░').collect();
        assert_eq!(solid, cells(&block));
        assert!(shadow.cells.iter().any(|c| c.2 == '░'));

        let line = Art::from_text_with("Hola", "mundo", Font::Outline).unwrap();
        assert_consistent(&line);
        assert_eq!((line.w, line.h), (block.w, block.h));
        // El contorno vacía el interior y nunca dibuja fuera de la letra.
        assert!(line.cells.len() < block.cells.len());
        let filled: std::collections::HashSet<(i32, i32)> = block.cells.iter().map(|c| (c.0, c.1)).collect();
        assert!(line.cells.iter().all(|c| filled.contains(&(c.0, c.1))));
        for font in [Font::Shadow, Font::Outline, Font::Pixel] {
            assert!(Art::from_text_with("", "", font).is_none());
            assert!(Art::from_text_with("😀", "", font).is_none());
        }
    }

    #[test]
    fn from_text_two_lines() {
        let t = Art::from_text("JACC", "").unwrap();
        let a = Art::from_text("JACC", "xacode").unwrap();
        assert_consistent(&a);
        assert!(a.h > t.h);
        // Título arriba (bloques ██), subtítulo debajo en medios bloques, con una fila vacía entre ambos.
        let title_bottom = a.cells.iter().filter(|c| c.1 < t.h).map(|c| c.1).max().unwrap();
        let sub_top = a.cells.iter().filter(|c| c.1 >= t.h).map(|c| c.1).min().unwrap();
        assert!(sub_top >= title_bottom + 2, "{title_bottom} {sub_top}");
        assert!(a.cells.iter().filter(|c| c.1 >= t.h).any(|c| c.2 == '▀' || c.2 == '▄'));
        // Ambos bloques centrados uno respecto al otro (±1 celda).
        let span = |f: &dyn Fn(&(i32, i32, char)) -> bool| {
            let xs: Vec<i32> = a.cells.iter().filter(|c| f(c)).map(|c| c.0).collect();
            xs.iter().min().unwrap() + xs.iter().max().unwrap()
        };
        let (top, bot) = (span(&|c| c.1 < t.h), span(&|c| c.1 >= t.h));
        assert!((top - bot).abs() <= 3, "{top} vs {bot}");
    }

    #[test]
    fn from_text_trims_and_truncates() {
        let cells_of = |t: &str, s: &str| Art::from_text(t, s).map(|a| (a.w, a.h, a.cells));
        assert_eq!(cells_of("  Hi  ", "  yo "), cells_of("Hi", "yo"));
        let w40 = "W".repeat(40);
        assert_eq!(cells_of(&w40, ""), cells_of(&"W".repeat(MAX_TITLE), ""));
        assert_eq!(cells_of("A", &w40), cells_of("A", &"W".repeat(MAX_SUBTITLE)));
        // Solo subtítulo largo: pasa a título y se recorta a MAX_TITLE.
        assert_eq!(cells_of("", &w40), cells_of(&"W".repeat(MAX_TITLE), ""));
    }

    #[test]
    fn from_text_without_drawable_glyphs_is_none() {
        for (t, s) in [("", ""), ("   ", "\t"), ("😀", ""), ("日本", "中文"), ("\u{7}", "")] {
            assert!(Art::from_text(t, s).is_none(), "{t:?} {s:?}");
        }
        // Mezcla con algo dibujable sí vale.
        assert!(Art::from_text("A😀", "").is_some());
        assert!(Art::from_text("😀", "b").is_some());
    }

    #[test]
    fn from_text_unicode_latin_and_greek() {
        for t in ["Ñandú", "Ça va", "ÆØÅ", "Ωμέγα", "¡Hola!"] {
            let a = Art::from_text(t, t).unwrap_or_else(|| panic!("{t}"));
            assert_consistent(&a);
        }
    }

    #[test]
    fn from_text_max_length_fits_limits_for_every_ascii_char() {
        // El rótulo más ancho posible (16 + 32 de la letra más ancha) debe seguir siendo válido.
        for ch in '!'..='~' {
            let (t, s) = (ch.to_string().repeat(MAX_TITLE), ch.to_string().repeat(MAX_SUBTITLE));
            let a = Art::from_text(&t, &s).unwrap_or_else(|| panic!("{ch:?}"));
            assert_consistent(&a);
            assert!(a.h <= 9 + 2 + 4, "{ch:?}: {}", a.h);
            for font in [Font::Shadow, Font::Outline, Font::Pixel] {
                assert_consistent(&Art::from_text_with(&t, &s, font).unwrap_or_else(|| panic!("{ch:?}")));
            }
        }
        let all: String = ('!'..='~').collect();
        assert_consistent(&Art::from_text(&all, &all).unwrap());
    }

    #[test]
    fn banner_text_always_ends_in_newline_without_blank_edges() {
        for (t, s) in [("A", ""), ("", "b"), ("gjpq", "y"), ("Hola", "Mundo")] {
            let txt = banner_text(t, s);
            assert!(txt.ends_with('\n'));
            let lines: Vec<&str> = txt.trim_end_matches('\n').split('\n').collect();
            assert!(!lines.first().unwrap().is_empty() && !lines.last().unwrap().is_empty(), "{t:?} {s:?}");
            assert!(lines.iter().all(|l| l.trim_end() == *l), "sin espacios al final");
        }
    }
}
