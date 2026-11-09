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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{Rgb, WHITE};
    use crate::rng::Rng;

    const RED: Rgb = Rgb(255, 0, 0);

    /// Píxeles de la celda (cx, cy) como 16 filas de 8.
    fn cell_px(r: &Raster, cx: usize, cy: usize) -> Vec<Vec<u32>> {
        (0..CH).map(|y| r.px[(cy * CH + y) * r.w + cx * CW..][..CW].to_vec()).collect()
    }

    fn fresh(g: &Grid) -> Vec<u32> {
        let mut r = Raster::new(g.cols, g.rows);
        r.update(g);
        r.px
    }

    #[test]
    fn new_sizes() {
        let r = Raster::new(5, 3);
        assert_eq!((r.w, r.h), (5 * CW, 3 * CH));
        assert_eq!(r.px.len(), r.w * r.h);
        assert!(r.px.iter().all(|&p| p == 0));
        assert!(r.dirty.is_none());
        // Tamaño nulo o negativo: una celda como mínimo, sin pánico.
        for (c, rows) in [(0, 0), (-4, 2), (2, -4)] {
            let r = Raster::new(c, rows);
            assert!(r.w >= CW && r.h >= CH);
            assert_eq!(r.px.len(), r.w * r.h);
        }
    }

    #[test]
    fn empty_grid_changes_nothing() {
        let mut r = Raster::new(4, 3);
        assert!(!r.update(&Grid::new(4, 3)));
        assert!(r.dirty.is_none());
        assert!(r.px.iter().all(|&p| p == 0));
    }

    #[test]
    fn full_block_fills_exactly_its_cell() {
        let mut g = Grid::new(4, 3);
        g.put(2, 1, '█', RED);
        let mut r = Raster::new(4, 3);
        assert!(r.update(&g));
        assert_eq!(r.dirty, Some((2 * CW as u32, CH as u32, CW as u32, CH as u32)));
        for (i, &p) in r.px.iter().enumerate() {
            let (x, y) = (i % r.w, i / r.w);
            let inside = (2 * CW..3 * CW).contains(&x) && (CH..2 * CH).contains(&y);
            assert_eq!(p, if inside { 0xFF0000 } else { 0 }, "({x},{y})");
        }
        // Mismo contenido otra vez: nada que redibujar.
        assert!(!r.update(&g));
        assert!(r.dirty.is_none());
        // Borrar la celda la deja en negro.
        g.clear();
        assert!(r.update(&g));
        assert_eq!(r.dirty, Some((2 * CW as u32, CH as u32, CW as u32, CH as u32)));
        assert!(r.px.iter().all(|&p| p == 0));
    }

    #[test]
    fn color_change_only_redraws() {
        let mut g = Grid::new(2, 1);
        g.put(0, 0, '█', RED);
        let mut r = Raster::new(2, 1);
        r.update(&g);
        g.put(0, 0, '█', WHITE);
        assert!(r.update(&g));
        assert!(cell_px(&r, 0, 0).iter().flatten().all(|&p| p == 0xFFFFFF));
    }

    #[test]
    fn half_blocks_and_bit_order() {
        let mut g = Grid::new(4, 1);
        g.put(0, 0, '▀', WHITE);
        g.put(1, 0, '▄', WHITE);
        g.put(2, 0, '▌', WHITE);
        g.put(3, 0, '▐', WHITE);
        let mut r = Raster::new(4, 1);
        r.update(&g);
        let (top, bottom, left, right) = (cell_px(&r, 0, 0), cell_px(&r, 1, 0), cell_px(&r, 2, 0), cell_px(&r, 3, 0));
        for y in 0..CH {
            for x in 0..CW {
                let on = |b: bool| if b { 0xFFFFFF } else { 0 };
                assert_eq!(top[y][x], on(y < CH / 2), "▀ ({x},{y})");
                assert_eq!(bottom[y][x], on(y >= CH / 2), "▄ ({x},{y})");
                // bit 0 = columna izquierda
                assert_eq!(left[y][x], on(x < CW / 2), "▌ ({x},{y})");
                assert_eq!(right[y][x], on(x >= CW / 2), "▐ ({x},{y})");
            }
        }
        assert_eq!(r.dirty, Some((0, 0, 4 * CW as u32, CH as u32)));
    }

    #[test]
    fn block_element_masks() {
        assert_eq!(mask_for('█'), [0xFF; 16]);
        assert_eq!(mask_for('\u{2588}'), [0xFF; 16]);
        // Octavos inferiores: cada uno tiene 2 filas más que el anterior y crece desde abajo.
        for (k, ch) in ('\u{2581}'..='\u{2587}').enumerate() {
            let m = mask_for(ch);
            let on = m.iter().filter(|&&v| v == 0xFF).count();
            assert_eq!(on, 2 * (k + 1), "{ch}");
            assert_eq!(m[15], 0xFF, "{ch}");
            assert!(m.iter().all(|&v| v == 0 || v == 0xFF), "{ch}");
        }
        // Octavos izquierdos: de 7/8 a 1/8, anclados a la izquierda.
        for (k, ch) in ('\u{2589}'..='\u{258F}').enumerate() {
            let m = mask_for(ch);
            let cols = 7 - k as u32;
            assert!(m.iter().all(|&v| v.count_ones() == cols && v & 1 == 1), "{ch}: {m:?}");
        }
        // Cuadrantes: cada uno cubre 1 a 3 cuartos.
        for ch in '\u{2596}'..='\u{259F}' {
            let m = mask_for(ch);
            let quarters = [m[0] & 0x0F != 0, m[0] & 0xF0 != 0, m[8] & 0x0F != 0, m[8] & 0xF0 != 0];
            let n = quarters.iter().filter(|&&q| q).count();
            assert!((1..=3).contains(&n), "{ch}");
        }
        // Sombreados de menos a más denso.
        let ink = |ch| mask_for(ch).iter().map(|v| v.count_ones()).sum::<u32>();
        assert!(ink('░') < ink('▒') && ink('▒') < ink('▓') && ink('▓') < ink('█'));
    }

    #[test]
    fn every_printable_char_has_ink_and_nothing_panics() {
        for ch in '!'..='~' {
            assert!(mask_for(ch).iter().any(|&v| v != 0), "{ch:?} sin píxeles");
        }
        // Cualquier carácter da una máscara (los desconocidos, '?').
        let ranges = [0u32..0x80, 0xA0..0x250, 0x370..0x400, 0x2010..0x2030, 0x2190..0x2200, 0x2500..0x2600, 0x2700..0x2800, 0x3040..0x30A0];
        for ch in ranges.into_iter().flatten().filter_map(char::from_u32) {
            mask_for(ch);
        }
        assert_eq!(mask_for('😀'), mask_for('?'));
        assert_eq!(mask_for('中'), mask_for('?'));
        // Los símbolos que usan los efectos tienen píxeles propios.
        for ch in ['·', '∙', '•', '✶', '✦', '❯', '›', '░', '▒', '▓'] {
            assert!(mask_for(ch).iter().any(|&v| v != 0), "{ch}");
            assert_ne!(mask_for(ch), mask_for('?'), "{ch}");
        }
    }

    #[test]
    fn incremental_updates_match_a_fresh_render() {
        const CHARS: [char; 12] = ['█', '▀', '▄', 'A', 'z', '·', '•', '✶', '░', '?', 'ñ', '❯'];
        let (cols, rows) = (13, 7);
        let mut rng = Rng::new(5);
        let mut g = Grid::new(cols, rows);
        let mut r = Raster::new(cols, rows);
        let mut prev = r.px.clone();
        for frame in 0..300 {
            // Cambia unas pocas celdas y a veces borra todo.
            if frame % 50 == 49 {
                g.clear();
            }
            for _ in 0..rng.below(6) {
                let (x, y) = (rng.below(cols as usize) as i32, rng.below(rows as usize) as i32);
                let col = Rgb(rng.below(256) as u8, rng.below(256) as u8, rng.below(256) as u8);
                g.put(x, y, CHARS[rng.below(CHARS.len())], col);
            }
            let changed = r.update(&g);
            assert_eq!(r.px, fresh(&g), "fotograma {frame}");
            assert_eq!(changed, r.dirty.is_some());
            // Fuera del rectángulo sucio nada cambió.
            let (dx, dy, dw, dh) = r.dirty.map(|(x, y, w, h)| (x as usize, y as usize, w as usize, h as usize)).unwrap_or((0, 0, 0, 0));
            assert!(dx + dw <= r.w && dy + dh <= r.h);
            assert_eq!(dx % CW + dy % CH + dw % CW + dh % CH, 0, "rectángulo alineado a celdas");
            for y in 0..r.h {
                let (row, old) = (&r.px[y * r.w..][..r.w], &prev[y * r.w..][..r.w]);
                if (dy..dy + dh).contains(&y) {
                    assert_eq!(row[..dx], old[..dx], "fotograma {frame} fila {y}");
                    assert_eq!(row[dx + dw..], old[dx + dw..], "fotograma {frame} fila {y}");
                } else {
                    assert_eq!(row, old, "fotograma {frame} fila {y}");
                }
            }
            prev.copy_from_slice(&r.px);
        }
    }
}
