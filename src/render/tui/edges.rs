//! Edge rendering for TUI output.
//!
//! Renders edges as braille lines between nodes, with Unicode arrow tips
//! (▶▼◀▲) and text labels at midpoints.

use crate::layout::{LayoutEdge, LayoutGraph};

use super::canvas::BrailleCanvas;
use super::scale::CellScale;

/// Arrow tip characters for each cardinal direction.
const ARROW_RIGHT: char = '▶';
const ARROW_DOWN: char = '▼';
const ARROW_LEFT: char = '◀';
const ARROW_UP: char = '▲';

/// Context for edge rendering operations.
struct EdgeContext<'a> {
    scale: &'a CellScale,
    canvas_cols: usize,
    canvas_rows: usize,
    offset_x: f64,
    offset_y: f64,
}

/// Render all edges onto the canvas using braille sub-pixel lines.
///
/// `occupied` is a grid of booleans where `true` means a node occupies that cell.
/// Braille characters are only composited onto unoccupied cells.
#[allow(clippy::too_many_arguments)]
pub fn render_edges(
    graph: &LayoutGraph,
    scale: &CellScale,
    canvas_cols: usize,
    canvas_rows: usize,
    offset_x: f64,
    offset_y: f64,
    occupied: &[Vec<bool>],
    canvas: &mut [Vec<char>],
) {
    let ctx = EdgeContext {
        scale,
        canvas_cols,
        canvas_rows,
        offset_x,
        offset_y,
    };

    let mut braille = BrailleCanvas::new(canvas_cols, canvas_rows);

    for edge in &graph.edges {
        render_edge_to_braille(edge, &ctx, &mut braille);
    }

    // Composite braille onto canvas, skipping occupied cells
    let braille_grid = braille.to_char_grid();
    for (row, braille_row) in braille_grid.iter().enumerate() {
        if row >= canvas_rows {
            break;
        }
        for (col, &ch) in braille_row.iter().enumerate() {
            if col >= canvas_cols {
                break;
            }
            if ch != ' ' && !occupied[row][col] {
                canvas[row][col] = ch;
            }
        }
    }

    // Render arrow tips and edge labels on top
    for edge in &graph.edges {
        render_arrow_tip(edge, &ctx, canvas);
        render_edge_label(edge, &ctx, canvas);
    }
}

/// Render a single edge's polyline segments into the braille canvas.
fn render_edge_to_braille(edge: &LayoutEdge, ctx: &EdgeContext, braille: &mut BrailleCanvas) {
    if edge.bend_points.len() < 2 {
        return;
    }

    for pair in edge.bend_points.windows(2) {
        let (x0, y0) = to_braille_coords(
            pair[0].x - ctx.offset_x,
            pair[0].y - ctx.offset_y,
            ctx.scale,
        );
        let (x1, y1) = to_braille_coords(
            pair[1].x - ctx.offset_x,
            pair[1].y - ctx.offset_y,
            ctx.scale,
        );
        braille.draw_line(x0, y0, x1, y1);
    }
}

/// Convert pixel coordinates to braille sub-pixel coordinates.
/// Each cell is 2 braille pixels wide and 4 braille pixels tall.
fn to_braille_coords(px: f64, py: f64, scale: &CellScale) -> (isize, isize) {
    let bx = ((px / scale.cell_width) * 2.0).round() as isize;
    let by = ((py / scale.cell_height) * 4.0).round() as isize;
    (bx, by)
}

/// Render an arrow tip at the edge's endpoint.
fn render_arrow_tip(edge: &LayoutEdge, ctx: &EdgeContext, canvas: &mut [Vec<char>]) {
    if edge.bend_points.len() < 2 {
        return;
    }

    // Arrow at the last point, direction from second-to-last to last
    let n = edge.bend_points.len();
    let prev = &edge.bend_points[n - 2];
    let last = &edge.bend_points[n - 1];

    let dx = last.x - prev.x;
    let dy = last.y - prev.y;

    let arrow = arrow_direction(dx, dy);

    let col = ctx.scale.to_col(last.x - ctx.offset_x);
    let row = ctx.scale.to_row(last.y - ctx.offset_y);

    if row < ctx.canvas_rows && col < ctx.canvas_cols {
        canvas[row][col] = arrow;
    }
}

/// Render an edge label at its midpoint position.
fn render_edge_label(edge: &LayoutEdge, ctx: &EdgeContext, canvas: &mut [Vec<char>]) {
    let label = match &edge.label {
        Some(l) if !l.is_empty() => l,
        _ => return,
    };

    // Use label_position if available, otherwise midpoint of bend_points
    let (lx, ly) = if let Some(ref pos) = edge.label_position {
        (pos.x - ctx.offset_x, pos.y - ctx.offset_y)
    } else if edge.bend_points.len() >= 2 {
        let mid = edge.bend_points.len() / 2;
        let p = &edge.bend_points[mid];
        (p.x - ctx.offset_x, p.y - ctx.offset_y)
    } else {
        return;
    };

    let col = ctx.scale.to_col(lx);
    let row = ctx.scale.to_row(ly);

    // Center the label text
    let start_col = col.saturating_sub(label.len() / 2);

    if row < ctx.canvas_rows {
        for (i, ch) in label.chars().enumerate() {
            let c = start_col + i;
            if c < ctx.canvas_cols {
                canvas[row][c] = ch;
            }
        }
    }
}

/// Determine the cardinal direction of an arrow from a segment.
pub fn arrow_direction(dx: f64, dy: f64) -> char {
    if dx.abs() > dy.abs() {
        if dx > 0.0 {
            ARROW_RIGHT
        } else {
            ARROW_LEFT
        }
    } else if dy > 0.0 {
        ARROW_DOWN
    } else {
        ARROW_UP
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_direction_right() {
        assert_eq!(arrow_direction(10.0, 0.0), '▶');
    }

    #[test]
    fn arrow_direction_left() {
        assert_eq!(arrow_direction(-10.0, 0.0), '◀');
    }

    #[test]
    fn arrow_direction_down() {
        assert_eq!(arrow_direction(0.0, 10.0), '▼');
    }

    #[test]
    fn arrow_direction_up() {
        assert_eq!(arrow_direction(0.0, -10.0), '▲');
    }

    #[test]
    fn arrow_direction_diagonal_mostly_right() {
        assert_eq!(arrow_direction(10.0, 3.0), '▶');
    }

    #[test]
    fn arrow_direction_diagonal_mostly_down() {
        assert_eq!(arrow_direction(3.0, 10.0), '▼');
    }

    #[test]
    fn braille_coords_origin() {
        let scale = CellScale::default();
        let (bx, by) = to_braille_coords(0.0, 0.0, &scale);
        assert_eq!(bx, 0);
        assert_eq!(by, 0);
    }

    #[test]
    fn braille_coords_one_cell() {
        let scale = CellScale::default();
        // One cell = 8px wide, 16px tall
        // In braille: 2 dots wide, 4 dots tall
        let (bx, by) = to_braille_coords(8.0, 16.0, &scale);
        assert_eq!(bx, 2);
        assert_eq!(by, 4);
    }
}
