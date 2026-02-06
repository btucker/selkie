//! ASCII renderer for kanban board diagrams.
//!
//! Renders kanban columns side-by-side in a horizontal layout,
//! using box-drawing characters for column headers and card borders.

use crate::diagrams::kanban::KanbanDb;
use crate::error::Result;

/// Minimum content width for a column (characters).
const MIN_COL_WIDTH: usize = 20;

/// Render a single kanban column into its own lines.
fn render_column(db: &KanbanDb, section: &crate::diagrams::kanban::KanbanNode) -> Vec<String> {
    let children = db.get_children(&section.id);

    let header = &section.label;
    let col_width = header.chars().count().max(MIN_COL_WIDTH);

    let mut lines = Vec::new();

    // Top border
    lines.push(format!("┌{}┐", "─".repeat(col_width + 2)));

    // Header (centered)
    let pad_total = col_width.saturating_sub(header.chars().count());
    let pad_left = pad_total / 2;
    let pad_right = pad_total - pad_left;
    lines.push(format!(
        "│ {}{}{} │",
        " ".repeat(pad_left),
        header,
        " ".repeat(pad_right)
    ));

    // Separator
    lines.push(format!("├{}┤", "─".repeat(col_width + 2)));

    // Cards
    if children.is_empty() {
        lines.push(format!("│ {:width$} │", "(empty)", width = col_width));
    } else {
        for child in &children {
            let label = &child.label;
            let card_text = if label.chars().count() > col_width {
                let truncated: String = label.chars().take(col_width - 1).collect();
                format!("{}…", truncated)
            } else {
                format!("{:width$}", label, width = col_width)
            };
            lines.push(format!("│ {} │", card_text));
        }
    }

    // Bottom border
    lines.push(format!("└{}┘", "─".repeat(col_width + 2)));

    lines
}

/// Render a kanban board as character art with columns side-by-side.
pub fn render_kanban_ascii(db: &KanbanDb) -> Result<String> {
    let sections = db.get_sections();
    if sections.is_empty() {
        return Ok("(empty kanban board)\n".to_string());
    }

    // Render each column independently
    let mut columns: Vec<Vec<String>> = sections.iter().map(|s| render_column(db, s)).collect();

    // Find the max height across all columns and pad shorter ones
    // by inserting blank card rows before the bottom border
    let max_height = columns.iter().map(|c| c.len()).max().unwrap_or(0);
    for col in &mut columns {
        while col.len() < max_height {
            // Determine column content width from the top border line
            // Top border is "┌──...──┐", inner width = total chars - 2 (for ┌ and ┐)
            let inner_width = col.first().map_or(MIN_COL_WIDTH + 2, |line| {
                line.chars().count().saturating_sub(2)
            });
            let blank_row = format!("│{}│", " ".repeat(inner_width));
            // Insert before the last line (the bottom border └...┘)
            let insert_pos = col.len() - 1;
            col.insert(insert_pos, blank_row);
        }
    }

    // Compute the display width of each column (from its first line)
    let col_widths: Vec<usize> = columns
        .iter()
        .map(|c| c.first().map_or(0, |line| line.chars().count()))
        .collect();

    // Zip columns horizontally, padding shorter columns with blank space
    let mut output_lines: Vec<String> = Vec::with_capacity(max_height);
    for row in 0..max_height {
        let mut parts: Vec<String> = Vec::with_capacity(columns.len());
        for (col_idx, col) in columns.iter().enumerate() {
            if row < col.len() {
                parts.push(col[row].clone());
            } else {
                // Pad with spaces to match column width
                parts.push(" ".repeat(col_widths[col_idx]));
            }
        }
        output_lines.push(parts.join(" "));
    }

    Ok(output_lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_kanban() {
        let db = KanbanDb::new();
        let output = render_kanban_ascii(&db).unwrap();
        assert!(output.contains("empty kanban"));
    }

    #[test]
    fn gallery_kanban_renders() {
        let input = std::fs::read_to_string("docs/sources/kanban.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Kanban(db) => db,
            _ => panic!("Expected kanban"),
        };
        let output = render_kanban_ascii(&db).unwrap();
        assert!(output.contains("Todo"), "Output:\n{}", output);
        assert!(
            output.contains("Create Documentation"),
            "Output:\n{}",
            output
        );
        assert!(output.contains("In Progress"), "Output:\n{}", output);
    }

    #[test]
    fn columns_render_side_by_side() {
        let input = std::fs::read_to_string("docs/sources/kanban.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Kanban(db) => db,
            _ => panic!("Expected kanban"),
        };
        let output = render_kanban_ascii(&db).unwrap();
        // The first line should contain two top-left corners (one per column)
        let first_line = output.lines().next().unwrap();
        let corner_count = first_line.matches('┌').count();
        assert_eq!(
            corner_count, 2,
            "Expected 2 columns side-by-side on first line, got {}.\nFirst line: {}\nFull output:\n{}",
            corner_count, first_line, output
        );
    }

    #[test]
    fn columns_padded_to_same_height() {
        // Columns with different numbers of cards should be padded to the same height
        let input = std::fs::read_to_string("docs/sources/kanban.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Kanban(db) => db,
            _ => panic!("Expected kanban"),
        };
        let output = render_kanban_ascii(&db).unwrap();
        // All lines should have the same visual width (no ragged right edges)
        let lines: Vec<&str> = output.lines().collect();
        // The last line of each column should have └ on the same output line
        let last_line = lines.last().unwrap();
        let bottom_count = last_line.matches('└').count();
        assert_eq!(
            bottom_count, 2,
            "Expected 2 bottom-left corners on the same line.\nLast line: {}\nFull output:\n{}",
            last_line, output
        );
    }

    #[test]
    fn complex_kanban_all_columns_side_by_side() {
        let input = std::fs::read_to_string("docs/sources/kanban_complex.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Kanban(db) => db,
            _ => panic!("Expected kanban"),
        };
        let output = render_kanban_ascii(&db).unwrap();
        // First line should have 6 top-left corners (6 columns in complex kanban)
        let first_line = output.lines().next().unwrap();
        let corner_count = first_line.matches('┌').count();
        assert_eq!(
            corner_count, 6,
            "Expected 6 columns side-by-side, got {}.\nFirst line: {}\nFull output:\n{}",
            corner_count, first_line, output
        );
    }

    #[test]
    fn columns_use_box_drawing() {
        let input = std::fs::read_to_string("docs/sources/kanban.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Kanban(db) => db,
            _ => panic!("Expected kanban"),
        };
        let output = render_kanban_ascii(&db).unwrap();
        assert!(output.contains('┌'), "Output:\n{}", output);
        assert!(output.contains('┘'), "Output:\n{}", output);
        assert!(output.contains('├'), "Output:\n{}", output);
    }
}
