//! Braille canvas for sub-cell resolution edge rendering.
//!
//! Unicode braille characters (U+2800–U+28FF) encode a 2×4 dot grid per
//! character cell. Each dot maps to a bit in the character's offset from
//! U+2800, giving 2x horizontal and 4x vertical resolution compared to
//! regular character cells.
//!
//! Dot positions and their bit values:
//! ```text
//!   col 0  col 1
//!   ┌────┬────┐
//!   │ 0x01│ 0x08│  row 0
//!   │ 0x02│ 0x10│  row 1
//!   │ 0x04│ 0x20│  row 2
//!   │ 0x40│ 0x80│  row 3
//!   └────┴────┘
//! ```

/// A braille canvas that maps sub-cell dots to Unicode braille characters.
#[derive(Debug, Clone)]
pub struct BrailleCanvas {
    /// Width in character cells
    cell_cols: usize,
    /// Height in character cells
    cell_rows: usize,
    /// Dot buffer: one byte per cell, bits represent dots
    dots: Vec<u8>,
}

/// Bit values for each dot position within a braille cell.
/// Index: [row][col] where row is 0-3 and col is 0-1.
const BRAILLE_DOTS: [[u8; 2]; 4] = [
    [0x01, 0x08], // row 0
    [0x02, 0x10], // row 1
    [0x04, 0x20], // row 2
    [0x40, 0x80], // row 3
];

/// Unicode braille base codepoint (empty braille pattern).
const BRAILLE_BASE: u32 = 0x2800;

impl BrailleCanvas {
    /// Create a new canvas with the given dimensions in character cells.
    pub fn new(cell_cols: usize, cell_rows: usize) -> Self {
        Self {
            cell_cols,
            cell_rows,
            dots: vec![0; cell_cols * cell_rows],
        }
    }

    /// Sub-pixel width (2x cell columns).
    pub fn pixel_width(&self) -> usize {
        self.cell_cols * 2
    }

    /// Sub-pixel height (4x cell rows).
    pub fn pixel_height(&self) -> usize {
        self.cell_rows * 4
    }

    /// Set a dot at sub-pixel coordinates (px, py).
    /// Returns false if out of bounds.
    pub fn set(&mut self, px: usize, py: usize) -> bool {
        if px >= self.pixel_width() || py >= self.pixel_height() {
            return false;
        }
        let cell_col = px / 2;
        let cell_row = py / 4;
        let dot_col = px % 2;
        let dot_row = py % 4;
        let idx = cell_row * self.cell_cols + cell_col;
        self.dots[idx] |= BRAILLE_DOTS[dot_row][dot_col];
        true
    }

    /// Draw a line from (x0, y0) to (x1, y1) using Bresenham's algorithm.
    /// Coordinates are in sub-pixel space.
    pub fn draw_line(&mut self, x0: isize, y0: isize, x1: isize, y1: isize) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx: isize = if x0 < x1 { 1 } else { -1 };
        let sy: isize = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut cx = x0;
        let mut cy = y0;

        loop {
            if cx >= 0 && cy >= 0 {
                self.set(cx as usize, cy as usize);
            }
            if cx == x1 && cy == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                if cx == x1 {
                    break;
                }
                err += dy;
                cx += sx;
            }
            if e2 <= dx {
                if cy == y1 {
                    break;
                }
                err += dx;
                cy += sy;
            }
        }
    }

    /// Get the braille character for the cell at (col, row).
    pub fn get_char(&self, col: usize, row: usize) -> char {
        if col >= self.cell_cols || row >= self.cell_rows {
            return ' ';
        }
        let idx = row * self.cell_cols + col;
        let bits = self.dots[idx];
        if bits == 0 {
            ' '
        } else {
            char::from_u32(BRAILLE_BASE + bits as u32).unwrap_or(' ')
        }
    }

    /// Render the canvas to a 2D grid of characters.
    pub fn to_char_grid(&self) -> Vec<Vec<char>> {
        let mut grid = Vec::with_capacity(self.cell_rows);
        for row in 0..self.cell_rows {
            let mut line = Vec::with_capacity(self.cell_cols);
            for col in 0..self.cell_cols {
                line.push(self.get_char(col, row));
            }
            grid.push(line);
        }
        grid
    }

    /// Get canvas dimensions in cells.
    pub fn cell_dims(&self) -> (usize, usize) {
        (self.cell_cols, self.cell_rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_canvas_all_spaces() {
        let canvas = BrailleCanvas::new(3, 2);
        let grid = canvas.to_char_grid();
        assert_eq!(grid.len(), 2);
        assert_eq!(grid[0].len(), 3);
        for row in &grid {
            for &ch in row {
                assert_eq!(ch, ' ');
            }
        }
    }

    #[test]
    fn single_dot_top_left() {
        let mut canvas = BrailleCanvas::new(1, 1);
        canvas.set(0, 0);
        // Bit 0x01 → braille U+2801 = ⠁
        assert_eq!(canvas.get_char(0, 0), '⠁');
    }

    #[test]
    fn single_dot_bottom_right() {
        let mut canvas = BrailleCanvas::new(1, 1);
        canvas.set(1, 3);
        // Bit 0x80 → braille U+2880 = ⢀
        assert_eq!(canvas.get_char(0, 0), '⢀');
    }

    #[test]
    fn multiple_dots_combine() {
        let mut canvas = BrailleCanvas::new(1, 1);
        canvas.set(0, 0); // 0x01
        canvas.set(1, 0); // 0x08
                          // Combined: 0x09 → U+2809 = ⠉
        assert_eq!(canvas.get_char(0, 0), '⠉');
    }

    #[test]
    fn all_dots_filled() {
        let mut canvas = BrailleCanvas::new(1, 1);
        for py in 0..4 {
            for px in 0..2 {
                canvas.set(px, py);
            }
        }
        // All bits: 0xFF → U+28FF = ⣿
        assert_eq!(canvas.get_char(0, 0), '⣿');
    }

    #[test]
    fn out_of_bounds_returns_false() {
        let mut canvas = BrailleCanvas::new(2, 2);
        assert!(!canvas.set(4, 0)); // x out of bounds (pixel_width = 4)
        assert!(!canvas.set(0, 8)); // y out of bounds (pixel_height = 8)
        assert!(canvas.set(3, 7)); // max valid coords
    }

    #[test]
    fn pixel_dimensions() {
        let canvas = BrailleCanvas::new(10, 5);
        assert_eq!(canvas.pixel_width(), 20);
        assert_eq!(canvas.pixel_height(), 20);
    }

    #[test]
    fn draw_horizontal_line() {
        let mut canvas = BrailleCanvas::new(3, 1);
        canvas.draw_line(0, 2, 5, 2);
        // All cells in row should have dots set
        for col in 0..3 {
            assert_ne!(canvas.get_char(col, 0), ' ', "col {} should have dots", col);
        }
    }

    #[test]
    fn draw_vertical_line() {
        let mut canvas = BrailleCanvas::new(1, 3);
        canvas.draw_line(0, 0, 0, 11);
        // All cells in column should have dots set
        for row in 0..3 {
            assert_ne!(canvas.get_char(0, row), ' ', "row {} should have dots", row);
        }
    }

    #[test]
    fn draw_diagonal_line() {
        let mut canvas = BrailleCanvas::new(3, 3);
        canvas.draw_line(0, 0, 5, 11);
        // At least the corner cells should have dots
        assert_ne!(canvas.get_char(0, 0), ' ');
        assert_ne!(canvas.get_char(2, 2), ' ');
    }

    #[test]
    fn dots_in_different_cells() {
        let mut canvas = BrailleCanvas::new(2, 1);
        canvas.set(0, 0); // cell (0,0)
        canvas.set(2, 0); // cell (1,0) — pixel x=2 maps to cell col 1
        assert_ne!(canvas.get_char(0, 0), ' ');
        assert_ne!(canvas.get_char(1, 0), ' ');
    }
}
