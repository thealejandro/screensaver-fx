//! Rejilla de celdas tipo terminal: carácter + color sobre fondo negro.

use crate::color::{Rgb, BLACK};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: u32,
}

pub const EMPTY: Cell = Cell { ch: ' ', fg: 0 };

pub struct Grid {
    pub cols: i32,
    pub rows: i32,
    pub cells: Vec<Cell>,
}

impl Grid {
    pub fn new(cols: i32, rows: i32) -> Grid {
        Grid { cols, rows, cells: vec![EMPTY; (cols.max(0) * rows.max(0)) as usize] }
    }

    pub fn clear(&mut self) {
        self.cells.fill(EMPTY);
    }

    #[inline]
    pub fn put(&mut self, x: i32, y: i32, ch: char, c: Rgb) {
        if x >= 0 && y >= 0 && x < self.cols && y < self.rows {
            let cell = if ch == ' ' || c == BLACK { EMPTY } else { Cell { ch, fg: c.u32() } };
            self.cells[(y * self.cols + x) as usize] = cell;
        }
    }

    pub fn text(&mut self, x: i32, y: i32, s: &str, c: Rgb) {
        for (i, ch) in s.chars().enumerate() {
            self.put(x + i as i32, y, ch, c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::WHITE;

    fn at(g: &Grid, x: i32, y: i32) -> Cell {
        g.cells[(y * g.cols + x) as usize]
    }

    #[test]
    fn new_and_clear() {
        let mut g = Grid::new(4, 3);
        assert_eq!(g.cells.len(), 12);
        assert!(g.cells.iter().all(|&c| c == EMPTY));
        g.put(1, 1, 'x', WHITE);
        g.clear();
        assert!(g.cells.iter().all(|&c| c == EMPTY));
        assert_eq!(g.cells.len(), 12);
    }

    #[test]
    fn negative_or_zero_size_is_empty() {
        for (c, r) in [(0, 0), (0, 5), (5, 0), (-3, 4), (4, -3), (-1, -1)] {
            let mut g = Grid::new(c, r);
            assert!(g.cells.is_empty(), "{c}x{r}");
            g.put(0, 0, 'x', WHITE);
            g.text(-2, 0, "hola", WHITE);
            g.clear();
        }
    }

    #[test]
    fn put_stores_char_and_color() {
        let mut g = Grid::new(3, 2);
        g.put(2, 1, 'Q', Rgb(1, 2, 3));
        assert_eq!(at(&g, 2, 1), Cell { ch: 'Q', fg: 0x010203 });
        // Sobrescribe.
        g.put(2, 1, 'R', WHITE);
        assert_eq!(at(&g, 2, 1), Cell { ch: 'R', fg: 0xFFFFFF });
    }

    #[test]
    fn space_or_black_is_empty() {
        let mut g = Grid::new(3, 1);
        g.put(0, 0, 'A', WHITE);
        g.put(0, 0, ' ', WHITE);
        assert_eq!(at(&g, 0, 0), EMPTY);
        g.put(1, 0, 'A', WHITE);
        g.put(1, 0, 'A', BLACK);
        assert_eq!(at(&g, 1, 0), EMPTY);
    }

    #[test]
    fn put_out_of_bounds_is_ignored() {
        let mut g = Grid::new(3, 2);
        for (x, y) in [(-1, 0), (0, -1), (3, 0), (0, 2), (i32::MIN, i32::MIN), (i32::MAX, i32::MAX), (i32::MAX, 0), (0, i32::MAX)] {
            g.put(x, y, '#', WHITE);
        }
        assert!(g.cells.iter().all(|&c| c == EMPTY));
    }

    #[test]
    fn text_is_clipped_and_counts_chars() {
        let mut g = Grid::new(4, 1);
        g.text(-1, 0, "ñandú", WHITE); // 5 caracteres, 7 bytes
        let s: String = g.cells.iter().map(|c| c.ch).collect();
        assert_eq!(s, "andú");
        g.clear();
        g.text(2, 0, "xyz", WHITE);
        let s: String = g.cells.iter().map(|c| c.ch).collect();
        assert_eq!(s, "  xy");
        // Fila fuera: nada.
        g.clear();
        g.text(0, 1, "abcd", WHITE);
        assert!(g.cells.iter().all(|&c| c == EMPTY));
    }
}
