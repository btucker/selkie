//! ASCII renderer for quadrant chart diagrams.
//!
//! Renders a 2x2 grid with quadrant labels, axis labels, and data points
//! positioned by their x,y coordinates.

use crate::diagrams::quadrant::QuadrantDb;
use crate::error::Result;

const GRID_WIDTH: usize = 40;
const GRID_HEIGHT: usize = 20;

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

    // Place data points
    for point in db.get_points() {
        let px = (point.x * (GRID_WIDTH - 1) as f64).round() as usize;
        let py = ((1.0 - point.y) * (GRID_HEIGHT - 1) as f64).round() as usize;
        let px = px.min(GRID_WIDTH - 1);
        let py = py.min(GRID_HEIGHT - 1);
        grid[py][px] = '●';
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
}
