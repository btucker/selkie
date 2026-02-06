//! Edge rendering for ASCII output.
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
/// Braille characters and arrow tips are only composited onto unoccupied cells
/// to avoid corrupting node box-drawing characters.
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

    // Render arrow tips and edge labels, respecting occupied cells
    for edge in &graph.edges {
        render_arrow_tip(edge, &ctx, occupied, canvas);
        render_edge_label(edge, &ctx, occupied, canvas);
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

/// Render an arrow tip at the edge's endpoint, avoiding occupied cells.
/// Walks backward along the edge direction to find an unoccupied cell.
fn render_arrow_tip(
    edge: &LayoutEdge,
    ctx: &EdgeContext,
    occupied: &[Vec<bool>],
    canvas: &mut [Vec<char>],
) {
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

    // Walk backward from the endpoint to find the first unoccupied cell
    let step_col: isize = if dx.abs() > dy.abs() {
        if dx > 0.0 {
            -1
        } else {
            1
        }
    } else {
        0
    };
    let step_row: isize = if dy.abs() >= dx.abs() {
        if dy > 0.0 {
            -1
        } else {
            1
        }
    } else {
        0
    };

    for step in 0..5 {
        let try_row = (row as isize + step_row * step) as usize;
        let try_col = (col as isize + step_col * step) as usize;
        if try_row < ctx.canvas_rows && try_col < ctx.canvas_cols && !occupied[try_row][try_col] {
            canvas[try_row][try_col] = arrow;
            return;
        }
    }
}

/// Render an edge label at its midpoint position.
///
/// If the ideal position overlaps occupied cells (node boxes, diamonds), the
/// label is shifted to nearby rows/columns to find a fully clear placement.
/// This prevents label truncation (e.g., "Invalid" appearing as "Inv").
fn render_edge_label(
    edge: &LayoutEdge,
    ctx: &EdgeContext,
    occupied: &[Vec<bool>],
    canvas: &mut [Vec<char>],
) {
    let raw_label = match &edge.label {
        Some(l) if !l.is_empty() => l.as_str(),
        _ => return,
    };
    // Clean HTML line breaks and normalize whitespace for ASCII display
    let cleaned = raw_label.replace("<br/>", " ").replace("<br>", " ");
    let label = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let label_len = label.chars().count();

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

    let ideal_col = ctx.scale.to_col(lx);
    let ideal_row = ctx.scale.to_row(ly);
    let ideal_start = ideal_col.saturating_sub(label_len / 2);

    // Try to place the label at a position where all characters fit
    // without overlapping occupied cells. Search nearby rows/cols.
    if let Some((place_row, place_col)) =
        find_clear_label_position(ideal_row, ideal_start, label_len, ctx, occupied)
    {
        for (i, ch) in label.chars().enumerate() {
            let c = place_col + i;
            if c < ctx.canvas_cols && !occupied[place_row][c] {
                canvas[place_row][c] = ch;
            }
        }
    }
}

/// Find a (row, start_col) where the entire label fits without overlapping
/// occupied cells. Searches outward from the ideal position.
fn find_clear_label_position(
    ideal_row: usize,
    ideal_col: usize,
    label_len: usize,
    ctx: &EdgeContext,
    occupied: &[Vec<bool>],
) -> Option<(usize, usize)> {
    let max_row_offset: isize = 5;
    let max_col_offset: isize = 10;

    // Check if a label placement is entirely clear of occupied cells
    let is_clear = |row: usize, start_col: usize| -> bool {
        if row >= ctx.canvas_rows || start_col + label_len > ctx.canvas_cols {
            return false;
        }
        (start_col..start_col + label_len).all(|c| !occupied[row][c])
    };

    // First try: exact ideal position
    if is_clear(ideal_row, ideal_col) {
        return Some((ideal_row, ideal_col));
    }

    // Spiral outward: try nearby rows first, then shift columns
    for dist in 1..=(max_row_offset.max(max_col_offset)) {
        // Try row offsets at ideal column
        if dist <= max_row_offset {
            for &dir in &[-1isize, 1] {
                let try_row = ideal_row as isize + dist * dir;
                if try_row >= 0
                    && (try_row as usize) < ctx.canvas_rows
                    && is_clear(try_row as usize, ideal_col)
                {
                    return Some((try_row as usize, ideal_col));
                }
            }
        }

        // Try column offsets at rows near ideal
        if dist <= max_col_offset {
            for &col_dir in &[-1isize, 1] {
                let try_col = ideal_col as isize + dist * col_dir;
                if try_col < 0 {
                    continue;
                }
                let c = try_col as usize;
                // Check ideal row and nearby rows
                for row_off in 0..=max_row_offset.min(dist) {
                    for &row_dir in &[-1isize, 1] {
                        let try_row = ideal_row as isize + row_off * row_dir;
                        if try_row >= 0
                            && (try_row as usize) < ctx.canvas_rows
                            && is_clear(try_row as usize, c)
                        {
                            return Some((try_row as usize, c));
                        }
                        if row_off == 0 {
                            break;
                        }
                    }
                }
            }
        }
    }

    None
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
