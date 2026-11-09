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
