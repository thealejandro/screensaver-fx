//! Rótulo: el texto que se anima. Por defecto JACC + XACODE (como generate-screensaver.py).

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
        Art::parse(&banner_text(title, subtitle))
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

pub fn banner_text(title: &str, subtitle: &str) -> String {
    let mut title: String = title.trim().chars().take(MAX_TITLE).collect();
    let mut subtitle: String = subtitle.trim().chars().take(MAX_SUBTITLE).collect();
    // Una sola línea (esté en el campo que esté) siempre va en letra grande.
    if title.is_empty() {
        title = subtitle.chars().take(MAX_TITLE).collect();
        subtitle.clear();
    }
    let mut blocks = Vec::new();
    if !title.is_empty() {
        blocks.push(bitmap_line(&title, true));
    }
    if !subtitle.is_empty() {
        blocks.push(bitmap_line(&subtitle, false));
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
    out.join("\n") + "\n"
}

const BIG_J: [&str; 9] = ["       ████", "       ████", "       ████", "       ████", "       ████", "       ████", "████   ████", "████   ████", " ▀███████▀ "];
const BIG_A: [&str; 9] = [" ▄███████▄ ", "████   ████", "████   ████", "████   ████", "████▄▄▄████", "████▀▀▀████", "████   ████", "████   ████", "████   ████"];
const BIG_C: [&str; 9] = [" ▄███████▄ ", "████   ████", "████   ▀▀▀▀", "████       ", "████       ", "████       ", "████   ▄▄▄▄", "████   ████", " ▀███████▀ "];
const SM_X: [&str; 4] = ["██  ██", " ▀██▀ ", " ▄██▄ ", "██  ██"];
const SM_A: [&str; 4] = ["▄████▄", "██  ██", "██▀▀██", "██  ██"];
const SM_C: [&str; 4] = ["▄████▄", "██  ▀▀", "██  ▄▄", "▀████▀"];
const SM_O: [&str; 4] = ["▄████▄", "██  ██", "██  ██", "▀████▀"];
const SM_D: [&str; 4] = ["█████▄", "██  ██", "██  ██", "█████▀"];
const SM_E: [&str; 4] = ["██████", "██▄▄  ", "██▀▀  ", "██████"];

fn render<const N: usize>(letters: &[&[&str; N]], gap: usize) -> Vec<String> {
    (0..N).map(|r| letters.iter().map(|l| l[r]).collect::<Vec<_>>().join(&" ".repeat(gap))).collect()
}

/// El mismo screensaver.txt que genera examples/generate-screensaver.py.
pub fn default_text() -> String {
    let name = render(&[&BIG_J, &BIG_A, &BIG_C, &BIG_C], 3);
    let logo = render(&[&SM_X, &SM_A, &SM_C, &SM_O, &SM_D, &SM_E], 2);
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
        // Letra grande: 8 filas como mucho, todas de bloque completo ("██").
        assert!(a.h <= 8);
        assert!(a.cells.iter().all(|&(_, _, ch)| ch == '█'));
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
            assert!(a.h <= 8 + 1 + 4, "{ch:?}: {}", a.h);
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
