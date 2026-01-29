//! Coordinate scaling: pixel-space → cell-space
//!
//! Dagre layout produces pixel coordinates. The TUI renderer needs to map
//! these into character-cell coordinates. Each cell has a configurable
//! pixel width and height (default 8×16, matching a typical monospace font
//! aspect ratio).

/// Configuration for pixel-to-cell coordinate mapping.
#[derive(Debug, Clone)]
pub struct CellScale {
    /// Pixel width of one character cell
    pub cell_width: f64,
    /// Pixel height of one character cell
    pub cell_height: f64,
}

impl Default for CellScale {
    fn default() -> Self {
        Self {
            cell_width: 8.0,
            cell_height: 16.0,
        }
    }
}

impl CellScale {
    /// Convert pixel x-coordinate to cell column.
    pub fn to_col(&self, px: f64) -> usize {
        (px / self.cell_width).round().max(0.0) as usize
    }

    /// Convert pixel y-coordinate to cell row.
    pub fn to_row(&self, px: f64) -> usize {
        (px / self.cell_height).round().max(0.0) as usize
    }

    /// Convert pixel width to cell width (always at least 1).
    pub fn to_cell_width(&self, px: f64) -> usize {
        (px / self.cell_width).round().max(1.0) as usize
    }

    /// Convert pixel height to cell height (always at least 1).
    pub fn to_cell_height(&self, px: f64) -> usize {
        (px / self.cell_height).round().max(1.0) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cell_size() {
        let scale = CellScale::default();
        assert_eq!(scale.cell_width, 8.0);
        assert_eq!(scale.cell_height, 16.0);
    }

    #[test]
    fn to_col_basic() {
        let scale = CellScale::default();
        assert_eq!(scale.to_col(0.0), 0);
        assert_eq!(scale.to_col(8.0), 1);
        assert_eq!(scale.to_col(16.0), 2);
        assert_eq!(scale.to_col(80.0), 10);
    }

    #[test]
    fn to_row_basic() {
        let scale = CellScale::default();
        assert_eq!(scale.to_row(0.0), 0);
        assert_eq!(scale.to_row(16.0), 1);
        assert_eq!(scale.to_row(32.0), 2);
    }

    #[test]
    fn rounds_to_nearest() {
        let scale = CellScale::default();
        // 12.0 / 8.0 = 1.5 → rounds to 2
        assert_eq!(scale.to_col(12.0), 2);
        // 4.0 / 8.0 = 0.5 → rounds to 0 (banker's rounding) or 1
        // Rust's f64::round() rounds 0.5 away from zero → 1
        assert_eq!(scale.to_col(4.0), 1);
    }

    #[test]
    fn negative_clamps_to_zero() {
        let scale = CellScale::default();
        assert_eq!(scale.to_col(-10.0), 0);
        assert_eq!(scale.to_row(-10.0), 0);
    }

    #[test]
    fn cell_width_at_least_one() {
        let scale = CellScale::default();
        assert_eq!(scale.to_cell_width(0.0), 1);
        assert_eq!(scale.to_cell_width(1.0), 1);
        assert_eq!(scale.to_cell_width(8.0), 1);
        assert_eq!(scale.to_cell_width(16.0), 2);
    }

    #[test]
    fn cell_height_at_least_one() {
        let scale = CellScale::default();
        assert_eq!(scale.to_cell_height(0.0), 1);
        assert_eq!(scale.to_cell_height(16.0), 1);
        assert_eq!(scale.to_cell_height(32.0), 2);
    }
}
