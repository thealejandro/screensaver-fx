//! Rasterizador: pinta solo las celdas que cambiaron en un búfer BGRX de 8x16 px por celda.
//! Bloques y símbolos se dibujan a mano para que encajen sin huecos; el resto usa font8x8.

use crate::grid::{Cell, Grid, EMPTY};
use font8x8::UnicodeFonts;
use std::collections::HashMap;

pub const CW: usize = 8;
pub const CH: usize = 16;

/// Máscara de 16 filas; bit i = columna i (bit 0 a la izquierda, como font8x8).
type Mask = [u8; 16];

pub struct Raster {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u32>,
    cols: usize,
    prev: Vec<Cell>,
    masks: HashMap<char, Mask>,
    /// Rectángulo cambiado en el último update (x, y, w, h) en px del búfer.
    pub dirty: Option<(u32, u32, u32, u32)>,
}

impl Raster {
    pub fn new(cols: i32, rows: i32) -> Raster {
        let (cols, rows) = (cols.max(1) as usize, rows.max(1) as usize);
        Raster {
            w: cols * CW,
            h: rows * CH,
            px: vec![0; cols * CW * rows * CH],
            cols,
            prev: vec![EMPTY; cols * rows],
            masks: HashMap::new(),
            dirty: None,
        }
    }

    /// Vuelca la rejilla al búfer. Devuelve true si cambió algún píxel.
    pub fn update(&mut self, g: &Grid) -> bool {
        let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
        for (i, cell) in g.cells.iter().enumerate() {
            if *cell == self.prev[i] {
                continue;
            }
            self.prev[i] = *cell;
            let (cx, cy) = (i % self.cols, i / self.cols);
            self.draw_cell(cx, cy, *cell);
            x0 = x0.min(cx);
            y0 = y0.min(cy);
            x1 = x1.max(cx + 1);
            y1 = y1.max(cy + 1);
        }
        if x0 == usize::MAX {
            self.dirty = None;
            return false;
        }
        self.dirty = Some(((x0 * CW) as u32, (y0 * CH) as u32, ((x1 - x0) * CW) as u32, ((y1 - y0) * CH) as u32));
        true
    }

    fn draw_cell(&mut self, cx: usize, cy: usize, cell: Cell) {
        let base = cy * CH * self.w + cx * CW;
        if cell.ch == ' ' {
            for r in 0..CH {
                self.px[base + r * self.w..base + r * self.w + CW].fill(0);
            }
            return;
        }
        let m = *self.masks.entry(cell.ch).or_insert_with(|| mask_for(cell.ch));
        for (r, bits) in m.iter().enumerate() {
            let row = &mut self.px[base + r * self.w..base + r * self.w + CW];
            for (i, p) in row.iter_mut().enumerate() {
                *p = if bits >> i & 1 == 1 { cell.fg } else { 0 };
            }
        }
    }
}

/// Bitmap de 8x8 escrito como texto ('#' encendido), colocado en las filas 4..12.
fn bitmap8(rows: [&str; 8]) -> Mask {
    let mut m = [0u8; 16];
    for (r, s) in rows.iter().enumerate() {
        m[r + 4] = s.bytes().enumerate().fold(0, |acc, (i, b)| if b == b'#' { acc | 1 << i } else { acc });
    }
    m
}

fn fill(m: &mut Mask, r0: usize, r1: usize, bits: u8) {
    m[r0..r1].iter_mut().for_each(|v| *v = bits);
}

pub fn mask_for(ch: char) -> Mask {
    let mut m = [0u8; 16];
    let u = ch as u32;
    match ch {
        '█' => fill(&mut m, 0, 16, 0xFF),
        '▀' => fill(&mut m, 0, 8, 0xFF),
        '▄' => fill(&mut m, 8, 16, 0xFF),
        '▌' => fill(&mut m, 0, 16, 0x0F),
        '▐' => fill(&mut m, 0, 16, 0xF0),
        '▔' => fill(&mut m, 0, 2, 0xFF),
        '▕' => fill(&mut m, 0, 16, 0x80),
        // ▁▂▃▄▅▆▇: octavos inferiores
        '\u{2581}'..='\u{2587}' => fill(&mut m, 16 - 2 * (u - 0x2580) as usize, 16, 0xFF),
        // ▉▊▋▌▍▎▏: octavos izquierdos
        '\u{2589}'..='\u{258F}' => fill(&mut m, 0, 16, ((1u16 << (8 - (u - 0x2588))) - 1) as u8),
        '░' => (0..16).for_each(|r| {
            m[r] = match r % 4 {
                0 => 0x11,
                2 => 0x44,
                _ => 0,
            }
        }),
        '▒' => (0..16).for_each(|r| m[r] = if r % 2 == 0 { 0x55 } else { 0xAA }),
        '▓' => (0..16).for_each(|r| {
            m[r] = match r % 4 {
                0 => 0xEE,
                2 => 0xBB,
                _ => 0xFF,
            }
        }),
        '\u{2596}'..='\u{259F}' => {
            // Cuadrantes ▖▗▘▙▚▛▜▝▞▟: (arriba-izq, arriba-der, abajo-izq, abajo-der)
            const Q: [(bool, bool, bool, bool); 10] = [
                (false, false, true, false),
                (false, false, false, true),
                (true, false, false, false),
                (true, false, true, true),
                (true, false, false, true),
                (true, true, true, false),
                (true, true, false, true),
                (false, true, false, false),
                (false, true, true, false),
                (false, true, true, true),
            ];
            let (tl, tr, bl, br) = Q[(u - 0x2596) as usize];
            let bits = |l: bool, r: bool| (if l { 0x0F } else { 0 }) | (if r { 0xF0 } else { 0 });
            fill(&mut m, 0, 8, bits(tl, tr));
            fill(&mut m, 8, 16, bits(bl, br));
        }
        '·' | '∙' => fill(&mut m, 7, 9, 0x18),
        '•' => {
            m[6] = 0x18;
            m[7] = 0x3C;
            m[8] = 0x3C;
            m[9] = 0x18;
        }
        '✶' | '✦' => {
            m = bitmap8(["...##...", "#..##..#", ".######.", "..####..", "..####..", ".######.", "#..##..#", "...##..."]);
        }
        '❯' | '›' => {
            m = bitmap8(["##......", ".##.....", "..##....", "...##...", "..##....", ".##.....", "##......", "........"]);
            m.iter_mut().for_each(|v| *v <<= 2);
        }
        _ => {
            let g = font8x8::BASIC_FONTS
                .get(ch)
                .or_else(|| font8x8::LATIN_FONTS.get(ch))
                .or_else(|| font8x8::BOX_FONTS.get(ch))
                .or_else(|| font8x8::BLOCK_FONTS.get(ch))
                .or_else(|| font8x8::GREEK_FONTS.get(ch))
                .or_else(|| font8x8::MISC_FONTS.get(ch))
                .or_else(|| font8x8::HIRAGANA_FONTS.get(ch))
                .or_else(|| font8x8::BASIC_FONTS.get('?'))
                .unwrap_or([0; 8]);
            // Doble alto: el aspecto 1:2 de una celda de terminal.
            for r in 0..8 {
                m[2 * r] = g[r];
                m[2 * r + 1] = g[r];
            }
        }
    }
    m
}
