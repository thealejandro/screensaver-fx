//! Pinta la rejilla del núcleo en la terminal con color de 24 bits, enviando solo las celdas
//! que cambiaron desde el fotograma anterior.

use ssfx_core::grid::{Cell, Grid, EMPTY};
use std::fmt::Write as _;
use std::io::{self, Write};

const CSI: &str = "\x1b[";

pub struct Term {
    cols: usize,
    prev: Vec<Cell>,
    buf: String,
}

impl Term {
    pub fn new(cols: i32, rows: i32) -> Term {
        let (cols, rows) = (cols.max(1) as usize, rows.max(1) as usize);
        Term { cols, prev: vec![EMPTY; cols * rows], buf: String::new() }
    }

    /// Secuencias que llevan la pantalla del fotograma anterior a `g` (vacío si no cambió nada).
    pub fn diff(&mut self, g: &Grid) -> &str {
        self.buf.clear();
        // Posición del cursor y color activos tras lo último escrito, si se conocen.
        let (mut at, mut fg) = (None, None);
        for (i, &cell) in g.cells.iter().enumerate().take(self.prev.len()) {
            if cell == self.prev[i] {
                continue;
            }
            self.prev[i] = cell;
            let (x, y) = (i % self.cols, i / self.cols);
            if at != Some(i) {
                let _ = write!(self.buf, "{CSI}{};{}H", y + 1, x + 1);
            }
            if cell.ch != ' ' && fg != Some(cell.fg) {
                let _ = write!(self.buf, "{CSI}38;2;{};{};{}m", (cell.fg >> 16) & 0xFF, (cell.fg >> 8) & 0xFF, cell.fg & 0xFF);
                fg = Some(cell.fg);
            }
            self.buf.push(cell.ch);
            // En la última columna la terminal deja el cursor «pendiente»: mejor recolocarlo.
            at = (x + 1 < self.cols).then_some(i + 1);
        }
        &self.buf
    }

    /// Pinta `g`. Con salida sincronizada la terminal muestra el fotograma entero de una vez.
    pub fn draw(&mut self, g: &Grid) -> io::Result<()> {
        if self.diff(g).is_empty() {
            return Ok(());
        }
        let mut out = io::stdout().lock();
        write!(out, "{CSI}?2026h{}{CSI}?2026l", self.buf)?;
        out.flush()
    }
}

/// Oculta el cursor y limpia la pantalla.
pub fn enter() -> io::Result<()> {
    let mut out = io::stdout().lock();
    write!(out, "{CSI}?25l{CSI}0m{CSI}2J")?;
    out.flush()
}

/// Restablece el color y, si `cursor`, vuelve a mostrar el cursor abajo a la izquierda.
pub fn leave(cursor: bool, rows: i32) {
    let mut out = io::stdout().lock();
    let _ = write!(out, "{CSI}0m");
    if cursor {
        let _ = write!(out, "{CSI}{};1H{CSI}?25h", rows.max(1));
    }
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use ssfx_core::color::Rgb;

    #[test]
    fn first_frame_then_only_changes() {
        let mut g = Grid::new(4, 2);
        let mut t = Term::new(4, 2);
        assert_eq!(t.diff(&g), "");
        g.put(1, 0, 'A', Rgb(1, 2, 3));
        g.put(2, 0, 'B', Rgb(1, 2, 3));
        // Dos celdas seguidas del mismo color: un salto, un color.
        assert_eq!(t.diff(&g), "\x1b[1;2H\x1b[38;2;1;2;3mAB");
        assert_eq!(t.diff(&g), "");
        // Borrar una celda escribe un espacio sin tocar el color.
        g.clear();
        g.put(1, 0, 'A', Rgb(1, 2, 3));
        assert_eq!(t.diff(&g), "\x1b[1;3H ");
    }

    #[test]
    fn last_column_forces_a_new_cursor_move() {
        let mut g = Grid::new(2, 2);
        let mut t = Term::new(2, 2);
        g.put(1, 0, 'X', Rgb(9, 9, 9));
        g.put(0, 1, 'Y', Rgb(9, 9, 9));
        assert_eq!(t.diff(&g), "\x1b[1;2H\x1b[38;2;9;9;9mX\x1b[2;1HY");
    }

    #[test]
    fn color_change_between_neighbours() {
        let mut g = Grid::new(3, 1);
        let mut t = Term::new(3, 1);
        g.put(0, 0, 'a', Rgb(255, 0, 0));
        g.put(1, 0, 'b', Rgb(0, 255, 0));
        assert_eq!(t.diff(&g), "\x1b[1;1H\x1b[38;2;255;0;0ma\x1b[38;2;0;255;0mb");
    }

    #[test]
    fn a_grid_of_another_size_never_panics() {
        let mut t = Term::new(2, 2);
        let mut big = Grid::new(10, 10);
        big.put(9, 9, 'Z', Rgb(1, 1, 1));
        big.put(0, 0, 'Q', Rgb(1, 1, 1));
        assert!(t.diff(&big).contains('Q'));
        assert!(!t.diff(&Grid::new(0, 0)).contains('Z'));
    }
}
