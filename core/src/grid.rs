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
