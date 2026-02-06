//! ASCII renderer for quadrant chart diagrams.
//!
//! Renders a 2x2 grid with quadrant labels, axis labels, and data points
//! positioned by their x,y coordinates.

use crate::diagrams::quadrant::QuadrantDb;
use crate::error::Result;
use log::warn;

const GRID_WIDTH: usize = 40;
const GRID_HEIGHT: usize = 20;

/// Find the nearest free cell for placing a point, starting from (x, y).
/// A cell is free if it contains only a space character.
/// Searches in a spiral pattern outward from the ideal position.
fn find_free_cell(grid: &[Vec<char>], x: usize, y: usize) -> Option<(usize, usize)> {
    let height = grid.len();
    let width = if height > 0 {
        grid[0].len()
    } else {
        return None;
    };

    if grid[y][x] == ' ' {
        return Some((x, y));
    }

    // Spiral search up to radius 10
    for r in 1..=10i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                // Only check cells on the perimeter of this radius
                if dy.abs() != r && dx.abs() != r {
                    continue;
                }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0
                    && ny >= 0
                    && (nx as usize) < width
                    && (ny as usize) < height
                    && grid[ny as usize][nx as usize] == ' '
                {
                    return Some((nx as usize, ny as usize));
                }
            }
        }
    }
    None
}

/// Render a quadrant chart as character art.
pub fn render_quadrant_ascii(db: &QuadrantDb) -> Result<String> {
    let mut lines: Vec<String> = Vec::new();

    // Title
    if !db.title.is_empty() {
        lines.push(db.title.clone());
        lines.push("─".repeat(db.title.chars().count().max(GRID_WIDTH + 6)));
    }

    // Y-axis top label
    if !db.y_axis_top.is_empty() {
        lines.push(format!("  ▲ {}", db.y_axis_top));
    }

    // Create the grid
    let mut grid: Vec<Vec<char>> = vec![vec![' '; GRID_WIDTH]; GRID_HEIGHT];

    // Draw quadrant dividers
    let mid_x = GRID_WIDTH / 2;
    let mid_y = GRID_HEIGHT / 2;
    for row in &mut grid {
        row[mid_x] = '│';
    }
    for cell in &mut grid[mid_y] {
        *cell = '─';
    }
    grid[mid_y][mid_x] = '┼';

    // Place quadrant labels
    let place_label = |grid: &mut Vec<Vec<char>>, text: &str, row: usize, col: usize| {
        for (i, ch) in text.chars().take(mid_x.saturating_sub(2)).enumerate() {
            if col + i < GRID_WIDTH && grid[row][col + i] == ' ' {
                grid[row][col + i] = ch;
            }
        }
    };

    // Quadrant positions (mermaid convention):
    // Q1 = top-right, Q2 = top-left, Q3 = bottom-left, Q4 = bottom-right
    let q_row_top = mid_y / 2;
    let q_row_bot = mid_y + mid_y / 2;
    if !db.quadrant1.is_empty() {
        place_label(&mut grid, &db.quadrant1, q_row_top, mid_x + 2);
    }
    if !db.quadrant2.is_empty() {
        place_label(&mut grid, &db.quadrant2, q_row_top, 1);
    }
    if !db.quadrant3.is_empty() {
        place_label(&mut grid, &db.quadrant3, q_row_bot, 1);
    }
    if !db.quadrant4.is_empty() {
        place_label(&mut grid, &db.quadrant4, q_row_bot, mid_x + 2);
    }

    // Place data points with collision avoidance
    for point in db.get_points() {
        let px = (point.x * (GRID_WIDTH - 1) as f64).round() as usize;
        let py = ((1.0 - point.y) * (GRID_HEIGHT - 1) as f64).round() as usize;
        let px = px.min(GRID_WIDTH - 1);
        let py = py.min(GRID_HEIGHT - 1);

        if let Some((fx, fy)) = find_free_cell(&grid, px, py) {
            grid[fy][fx] = '●';
        } else {
            warn!(
                "Quadrant point '{}' at ({:.2}, {:.2}) could not be placed: \
                 all cells within search radius are occupied",
                point.text, point.x, point.y
            );
        }
    }

    // Render grid with left border
    for row in &grid {
        let line: String = row.iter().collect();
        lines.push(format!("  │{}", line.trim_end()));
    }

    // X-axis
    lines.push(format!("  └{}▶", "─".repeat(GRID_WIDTH)));

    // X-axis labels
    if !db.x_axis_left.is_empty() || !db.x_axis_right.is_empty() {
        let left = &db.x_axis_left;
        let right = &db.x_axis_right;
        let gap = GRID_WIDTH.saturating_sub(left.chars().count() + right.chars().count());
        lines.push(format!("   {}{}{}", left, " ".repeat(gap), right));
    }

    // Y-axis bottom label
    if !db.y_axis_bottom.is_empty() {
        lines.push(format!("  ▼ {}", db.y_axis_bottom));
    }

    // Point legend
    let points = db.get_points();
    if !points.is_empty() {
        lines.push(String::new());
        lines.push("  Points:".to_string());
        for point in points {
            lines.push(format!(
                "    ● {} ({:.2}, {:.2})",
                point.text, point.x, point.y
            ));
        }
    }

    lines.push(String::new());
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_quadrant() {
        let db = QuadrantDb::new();
        let output = render_quadrant_ascii(&db).unwrap();
        assert!(!output.trim().is_empty());
    }

    #[test]
    fn gallery_quadrant_renders() {
        let input = std::fs::read_to_string("docs/sources/quadrant.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Quadrant(db) => db,
            _ => panic!("Expected quadrant"),
        };
        let output = render_quadrant_ascii(&db).unwrap();
        assert!(
            output.contains("Reach and Engagement"),
            "Output:\n{}",
            output
        );
        assert!(output.contains("Campaign A"), "Output:\n{}", output);
        assert!(output.contains("Campaign F"), "Output:\n{}", output);
    }

    #[test]
    fn quadrant_labels_appear() {
        let input = std::fs::read_to_string("docs/sources/quadrant.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Quadrant(db) => db,
            _ => panic!("Expected quadrant"),
        };
        let output = render_quadrant_ascii(&db).unwrap();
        assert!(output.contains("We should expand"), "Output:\n{}", output);
        assert!(output.contains("Re-evaluate"), "Output:\n{}", output);
    }

    #[test]
    fn has_grid_structure() {
        let input = std::fs::read_to_string("docs/sources/quadrant.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Quadrant(db) => db,
            _ => panic!("Expected quadrant"),
        };
        let output = render_quadrant_ascii(&db).unwrap();
        assert!(
            output.contains('┼'),
            "Should have center cross\nOutput:\n{}",
            output
        );
        assert!(
            output.contains('●'),
            "Should have data points\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn points_do_not_overwrite_quadrant_labels() {
        let input = std::fs::read_to_string("docs/sources/quadrant_complex.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Quadrant(db) => db,
            _ => panic!("Expected quadrant"),
        };
        let output = render_quadrant_ascii(&db).unwrap();
        // All four quadrant labels must appear intact
        assert!(
            output.contains("Leaders"),
            "Leaders label corrupted by points\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Challengers"),
            "Challengers label corrupted\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Niche Players"),
            "Niche Players label corrupted\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Visionaries"),
            "Visionaries label corrupted\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn points_do_not_overlap_axis_lines() {
        let input = std::fs::read_to_string("docs/sources/quadrant_complex.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Quadrant(db) => db,
            _ => panic!("Expected quadrant"),
        };
        let output = render_quadrant_ascii(&db).unwrap();
        // No line should have ● directly adjacent to ─ (point on horizontal axis)
        // and no line should have ● replacing │ on the vertical axis
        for line in output.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('│') {
                // Grid rows: the horizontal axis row uses ─ chars
                // Check that ● doesn't appear embedded in the axis line itself
                let content = trimmed.trim_start_matches('│');
                if content.contains('─') && content.contains('●') {
                    // This is the horizontal axis row - points should not be on it
                    panic!(
                        "Point rendered on horizontal axis line:\n{}\nFull output:\n{}",
                        line, output
                    );
                }
            }
        }
    }

    #[test]
    fn drops_points_when_placement_exhausted() {
        // Pack many points at the same coordinate to exhaust the radius-10 spiral search.
        // The grid is 40x20 = 800 cells, but axis lines, labels, and the border consume
        // many cells. Placing hundreds of points at the same spot will eventually exhaust
        // the search radius, and those unplaceable points are dropped (with a log warning).
        let mut db = QuadrantDb::new();
        for i in 0..500 {
            db.add_point(&format!("P{}", i), "", "0.50", "0.50", &[]);
        }
        let output = render_quadrant_ascii(&db).unwrap();
        let grid_bullets: usize = output
            .lines()
            .filter(|l| l.starts_with("  │"))
            .map(|l| l.chars().filter(|&c| c == '●').count())
            .sum();
        // With 500 points at the same spot, many will be dropped.
        // Verify that fewer points appear in the grid than were requested.
        assert!(
            grid_bullets < 500,
            "Some points should be dropped when search radius is exhausted\nOutput:\n{}",
            output
        );
        // Verify at least some points were placed
        assert!(
            grid_bullets > 0,
            "At least some points should be placed\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn nearby_points_do_not_share_cells() {
        // Create points that map very close together in the grid
        let mut db = QuadrantDb::new();
        db.set_quadrant1_text("Q1");
        db.add_point("A", "", "0.50", "0.50", &[]);
        db.add_point("B", "", "0.51", "0.50", &[]);
        let output = render_quadrant_ascii(&db).unwrap();
        // Count ● chars in grid (not legend). Legend always has them, so count
        // from the grid portion only (lines starting with "  │")
        let grid_bullets: usize = output
            .lines()
            .filter(|l| l.starts_with("  │"))
            .map(|l| l.chars().filter(|&c| c == '●').count())
            .sum();
        assert!(
            grid_bullets >= 2,
            "Two close points should both appear as ● in grid\nOutput:\n{}",
            output
        );
    }
}
