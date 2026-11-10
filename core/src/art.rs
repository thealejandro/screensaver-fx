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

    /// Como `from_text`, con el estilo de letra pedido.
    pub fn from_text_with(title: &str, subtitle: &str, font: Font) -> Option<Art> {
        Art::parse(&banner_text_with(title, subtitle, font))
    }

    /// JACC + XACODE con el estilo de letra pedido (la de píxel no aplica: es un dibujo a mano).
    pub fn default_art_with(font: Font) -> Art {
        Art::parse(&default_text_with(font)).expect("rótulo por defecto")
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

/// Dos píxeles, uno sobre otro, en una celda.
fn half(top: bool, bottom: bool) -> char {
    match (top, bottom) {
        (true, true) => '█',
        (true, false) => '▀',
        (false, true) => '▄',
        _ => ' ',
    }
}

/// Una letra lista para escribir: filas del mismo ancho y cuántas van hasta la línea base.
struct Glyph {
    rows: Vec<String>,
    base: usize,
}

fn designed(c: char, big: bool) -> Option<Vec<String>> {
    let rows: &[&str] = if big { font::big(c)? } else { font::small(c)? };
    Some(rows.iter().map(|r| r.to_string()).collect())
}

/// Letra con su signo encima (tilde, diéresis...), centrados uno sobre otro.
fn marked(rows: Vec<String>, mark: font::Mark, big: bool) -> Vec<String> {
    let px = mark.pixels(big);
    let (w, mw) = (rows[0].chars().count(), px[0].len());
    let width = w.max(mw);
    let pad = |s: &str, n: usize| format!("{}{s}{}", " ".repeat((width - n) / 2), " ".repeat(width - n - (width - n) / 2));
    // Una fila de píxeles libre entre el signo y la letra; arriba se completa hasta un número par.
    let mut lines: Vec<&str> = px.to_vec();
    lines.push("");
    if lines.len() % 2 == 1 {
        lines.insert(0, "");
    }
    let on = |l: &str, x: usize| l.as_bytes().get(x) == Some(&b'#');
    let mut out: Vec<String> = lines
        .chunks(2)
        .map(|p| pad(&(0..mw).map(|x| half(on(p[0], x), on(p[1], x))).collect::<String>(), mw))
        .collect();
    out.extend(rows.iter().map(|r| pad(r, w)));
    out
}

/// Letra de font8x8 recortada a su ancho, para lo que no tiene letra propia.
fn bitmap(c: char, big: bool) -> Glyph {
    let g = glyph(c);
    let used = g.iter().fold(0u8, |a, &r| a | r);
    let (c0, c1) = if used == 0 { (0, 3) } else { (used.trailing_zeros(), 7 - used.leading_zeros()) };
    let on = |row: usize, col: u32| g[row] >> col & 1 == 1;
    let rows: Vec<String> = if big {
        (0..8).map(|r| (c0..=c1).map(|c| if on(r, c) { "██" } else { "  " }).collect()).collect()
    } else {
        (0..4).map(|r| (c0..=c1).map(|c| half(on(r * 2, c), on(r * 2 + 1, c))).collect()).collect()
    };
    // La fila 8 de font8x8 es para lo que baja de la línea (ç, ¿...).
    Glyph { rows, base: if big { 7 } else { 4 } }
}

fn letter(c: char, big: bool, pixel: bool) -> Glyph {
    // Letra de píxel: todo sale de font8x8, tal cual se escribió (con sus minúsculas).
    if pixel {
        return bitmap(c, big);
    }
    // En mayúscula (si es una sola letra: ß se queda como está).
    let mut up = c.to_uppercase();
    let c = match (up.next(), up.next()) {
        (Some(u), None) => u,
        _ => c,
    };
    if let Some(rows) = designed(c, big) {
        return Glyph { base: rows.len(), rows };
    }
    if let Some((base, mark)) = font::decompose(c) {
        let rows = marked(designed(base, big).expect("letra base"), mark, big);
        return Glyph { base: rows.len(), rows };
    }
    bitmap(c, big)
}

/// Escribe `s` en bloques: `big` con las letras grandes (9 filas), si no con las pequeñas (4).
/// Las letras se alinean en la línea base; los signos de encima y lo que baja la hacen crecer.
fn bitmap_line(s: &str, big: bool, pixel: bool) -> Vec<String> {
    let glyphs: Vec<Glyph> = s.chars().map(|c| letter(c, big, pixel)).collect();
    let up = glyphs.iter().map(|g| g.base).max().unwrap_or(0);
    let down = glyphs.iter().map(|g| g.rows.len() - g.base).max().unwrap_or(0);
    // La letra de píxel va más apretada, como en las versiones anteriores.
    let gap = " ".repeat(match (big, pixel) {
        (true, false) => 3,
        (false, true) => 1,
        _ => 2,
    });
    let mut out = vec![String::new(); up + down];
    for (i, g) in glyphs.iter().enumerate() {
        let blank = " ".repeat(g.rows[0].chars().count());
        for (r, line) in out.iter_mut().enumerate() {
            if i > 0 {
                line.push_str(&gap);
            }
            let row = (r + g.base).checked_sub(up).and_then(|k| g.rows.get(k));
            line.push_str(row.unwrap_or(&blank));
        }
    }
    out
}

pub const MAX_TITLE: usize = 16;
pub const MAX_SUBTITLE: usize = 32;

fn width(s: &str) -> usize {
    s.chars().count()
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

/// Aplica el estilo de letra a un rótulo ya escrito en bloques.
fn styled(lines: Vec<String>, font: Font) -> Vec<String> {
    match font {
        Font::Shadow => add_shadow(&lines),
        Font::Outline => outline(&lines),
        Font::Block | Font::Pixel => lines,
    }
}

/// Rótulo de texto con la letra de bloque.
pub fn banner_text(title: &str, subtitle: &str) -> String {
    banner_text_with(title, subtitle, Font::Block)
}

pub fn banner_text_with(title: &str, subtitle: &str, font: Font) -> String {
    let pixel = font == Font::Pixel;
    let mut title: String = title.trim().chars().take(MAX_TITLE).collect();
    let mut subtitle: String = subtitle.trim().chars().take(MAX_SUBTITLE).collect();
    // Una sola línea (esté en el campo que esté) siempre va en letra grande.
    if title.is_empty() {
        title = subtitle.chars().take(MAX_TITLE).collect();
        subtitle.clear();
    }
    let mut blocks = Vec::new();
    if !title.is_empty() {
        blocks.push(bitmap_line(&title, true, pixel));
    }
    if !subtitle.is_empty() {
        blocks.push(bitmap_line(&subtitle, false, pixel));
    }
    let len = |s: &String| s.chars().count();
    let width = blocks.iter().map(|b| len(&b[0])).max().unwrap_or(0);
    let mut out: Vec<String> = Vec::new();
    for (i, b) in blocks.iter().enumerate() {
        if i > 0 {
            out.push(String::new());
        }
        let pad = " ".repeat((width - len(&b[0])) / 2);
        out.extend(b.iter().map(|l| format!("{pad}{l}").trim_end().to_string()));
    }
    // Quita filas vacías arriba y abajo (letras sin trazos en la base, como en minúsculas).
    while out.first().is_some_and(|l| l.is_empty()) {
        out.remove(0);
    }
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    styled(out, font).join("\n") + "\n"
}


fn render<const N: usize>(letters: &[&[&str; N]], gap: usize) -> Vec<String> {
    (0..N).map(|r| letters.iter().map(|l| l[r]).collect::<Vec<_>>().join(&" ".repeat(gap))).collect()
}

/// El rótulo por defecto con un estilo de letra.
pub fn default_text_with(font: Font) -> String {
    let lines: Vec<String> = default_text().trim_end_matches('\n').split('\n').map(str::to_string).collect();
    styled(lines, font).join("\n") + "\n"
}

/// El mismo screensaver.txt que genera examples/generate-screensaver.py.
pub fn default_text() -> String {
    let (b, s) = (|c| font::big(c).expect("letra"), |c| font::small(c).expect("letra"));
    let name = render(&[b('J'), b('A'), b('C'), b('C')], 3);
    let logo = render(&"XACODE".chars().map(s).collect::<Vec<_>>(), 2);
    let len = |s: &String| s.chars().count();
    let width = len(&name[0]).max(len(&logo[0]));
    let center = |lines: &[String]| -> Vec<String> {
        let pad = " ".repeat((width - len(&lines[0])) / 2);
        lines.iter().map(|l| format!("{pad}{l}").trim_end().to_string()).collect()
    };
    let mut out = center(&name);
    out.push(String::new());
    out.push(String::new());
    out.extend(center(&logo));
    out.join("\n") + "\n"
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
        // Letra grande: 9 filas, de bloques y medios bloques.
        assert_eq!(a.h, 9);
        assert!(a.cells.iter().all(|&(_, _, ch)| "█▀▄".contains(ch)));
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
            assert!(a.h <= 9 + 1 + 4, "{ch:?}: {}", a.h);
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
            let (t, s) = ("W".repeat(MAX_TITLE), "W".repeat(MAX_SUBTITLE));
            assert_consistent(&Art::from_text_with(&t, &s, font).unwrap());
        }
    }

    #[test]
    fn pixel_font_keeps_the_old_8x8_letters() {
        let p = Art::from_text_with("Hola", "", Font::Pixel).unwrap();
        assert!(p.h <= 8);
        assert!(p.cells.iter().all(|&(_, _, ch)| ch == '█'));
        // Conserva las minúsculas: no es el mismo dibujo que en mayúscula.
        assert_ne!(cells(&p), cells(&Art::from_text_with("HOLA", "", Font::Pixel).unwrap()));
        assert_eq!(cells(&Art::from_text("hola", "").unwrap()), cells(&Art::from_text("HOLA", "").unwrap()));
    }

    #[test]
    fn default_art_takes_the_font_style() {
        let plain = Art::default_art();
        assert_eq!(cells(&Art::default_art_with(Font::Block)), cells(&plain));
        assert_eq!(cells(&Art::default_art_with(Font::Pixel)), cells(&plain));
        let shadow = Art::default_art_with(Font::Shadow);
        assert_eq!((shadow.w, shadow.h), (plain.w + 1, plain.h + 1));
        assert!(Art::default_art_with(Font::Outline).cells.len() < plain.cells.len());
    }

    #[test]
    fn from_text_uses_own_letters_in_uppercase() {
        let a = banner_text("hola", "");
        assert_eq!(a, banner_text("HOLA", ""));
        // La S propia, no la de font8x8.
        let s: String = font::big('S').unwrap().map(|r| r.trim_end().to_string()).join("\n") + "\n";
        assert_eq!(banner_text("S", ""), s);
        assert_eq!(banner_text("ab", "cd"), banner_text("AB", "CD"));
    }

    #[test]
    fn from_text_accents_sit_above_the_letter() {
        let plain = Art::from_text("ANA", "").unwrap();
        for t in ["ÁNA", "AÑA", "ANÜ", "ÍNA"] {
            let a = Art::from_text(t, "").unwrap();
            assert_consistent(&a);
            // Dos filas más arriba; la base de las letras no se mueve.
            assert_eq!(a.h, plain.h + 2, "{t}");
            assert!(a.cells.iter().any(|c| c.1 < 2), "{t}: sin signo");
        }
        let a = Art::from_text("", "canción").unwrap();
        assert_eq!(a.h, 9 + 2);
        let a = Art::from_text("José", "María Núñez").unwrap();
        assert_consistent(&a);
        assert_eq!(a.h, 9 + 2 + 1 + 4 + 2);
    }

    #[test]
    fn from_text_fallback_glyphs_share_the_baseline() {
        // Æ sale de font8x8 y debe acabar en la misma fila que las letras propias.
        let a = Art::from_text("AÆ", "").unwrap();
        assert_consistent(&a);
        let bottom = |x0: i32, x1: i32| a.cells.iter().filter(|c| (x0..x1).contains(&c.0)).map(|c| c.1).max().unwrap();
        assert_eq!(bottom(0, 11), bottom(14, a.w));
        assert_eq!(a.h, 9);
    }
}
