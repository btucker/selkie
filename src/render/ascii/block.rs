//! ASCII renderer for block diagrams.
//!
//! Renders blocks in a grid layout with edges shown as flow connections.
//! Uses box-drawing characters for block borders and arrows for connections.
//! Supports shape differentiation (round, diamond, stadium) and nested composites.

use crate::diagrams::block::{BlockDb, BlockType};
use crate::error::Result;

/// Render a block diagram as character art.
pub fn render_block_ascii(db: &BlockDb) -> Result<String> {
    let blocks = db.get_blocks_flat();
    let edges = db.get_edges();

    if blocks.is_empty() {
        return Ok("(empty block diagram)\n".to_string());
    }

    let mut lines: Vec<String> = Vec::new();

    // Determine grid layout based on column settings
    let columns = db.get_columns().unwrap_or(3);

    // Filter to top-level blocks (not children of composites)
    let top_level: Vec<&str> = db
        .get_block_order()
        .iter()
        .filter(|id| {
            db.get_blocks()
                .get(id.as_str())
                .is_some_and(|b| b.parent_id.is_none())
        })
        .map(|s| s.as_str())
        .collect();

    // Calculate cell width from the longest label among all visible blocks
    let cell_width = calc_cell_width(db, &top_level);

    render_block_rows(db, &top_level, columns, cell_width, "  ", &mut lines);

    // Render edges as a connection list
    if !edges.is_empty() {
        lines.push("  Connections:".to_string());
        for edge in edges {
            let label_str = edge
                .label
                .as_ref()
                .map(|l| format!(" [{}]", l))
                .unwrap_or_default();
            lines.push(format!("    {} → {}{}", edge.start, edge.end, label_str));
        }
        lines.push(String::new());
    }

    Ok(lines.join("\n"))
}

/// Calculate the cell width needed for a set of block IDs.
fn calc_cell_width(db: &BlockDb, ids: &[&str]) -> usize {
    let max_label = ids
        .iter()
        .filter_map(|id| db.get_blocks().get(*id))
        .filter(|b| b.block_type != BlockType::Space)
        .map(|b| {
            b.label
                .as_ref()
                .map(|l| l.chars().count())
                .unwrap_or(b.id.chars().count())
        })
        .max()
        .unwrap_or(8);
    max_label + 4 // padding
}

/// Render rows of blocks at a given indent level.
fn render_block_rows(
    db: &BlockDb,
    ids: &[&str],
    columns: usize,
    cell_width: usize,
    indent: &str,
    lines: &mut Vec<String>,
) {
    for chunk in ids.chunks(columns) {
        // We need to know the max height for this row (composites are taller)
        let has_composite = chunk.iter().any(|id| {
            db.get_blocks()
                .get(*id)
                .is_some_and(|b| b.block_type == BlockType::Composite)
        });

        if has_composite {
            render_mixed_row(db, chunk, cell_width, indent, lines);
        } else {
            render_simple_row(db, chunk, cell_width, indent, lines);
        }

        lines.push(String::new());
    }
}

/// Render a row where all blocks are simple (non-composite).
fn render_simple_row(
    db: &BlockDb,
    chunk: &[&str],
    cell_width: usize,
    indent: &str,
    lines: &mut Vec<String>,
) {
    let blocks_map = db.get_blocks();

    // Top border
    let mut top = String::from(indent);
    for id in chunk {
        let block = blocks_map.get(*id);
        let bt = block.map(|b| &b.block_type).unwrap_or(&BlockType::Square);
        match bt {
            BlockType::Space => {
                top.push_str(&" ".repeat(cell_width + 2));
                top.push(' ');
            }
            BlockType::Round => {
                top.push_str(&format!("╭{}╮ ", "─".repeat(cell_width)));
            }
            BlockType::Diamond => {
                let total_pad = cell_width + 1;
                let left = total_pad / 2;
                let right = total_pad - left;
                top.push_str(&format!("{}◇{} ", " ".repeat(left), " ".repeat(right)));
            }
            BlockType::Stadium => {
                top.push_str(&format!("╭{}╮ ", "─".repeat(cell_width)));
            }
            _ => {
                top.push_str(&format!("┌{}┐ ", "─".repeat(cell_width)));
            }
        }
    }
    lines.push(top.trim_end().to_string());

    // Label row
    let mut label_line = String::from(indent);
    for id in chunk {
        let block = blocks_map.get(*id);
        let bt = block.map(|b| &b.block_type).unwrap_or(&BlockType::Square);

        if *bt == BlockType::Space {
            label_line.push_str(&" ".repeat(cell_width + 2));
            label_line.push(' ');
            continue;
        }

        let label = block
            .and_then(|b| b.label.as_ref())
            .map(|l| l.as_str())
            .unwrap_or(id);
        let pad_total = cell_width.saturating_sub(label.chars().count());
        let pad_left = pad_total / 2;
        let pad_right = pad_total - pad_left;

        match bt {
            BlockType::Diamond => {
                label_line.push_str(&format!(
                    "<{}{}{}>",
                    " ".repeat(pad_left),
                    label,
                    " ".repeat(pad_right)
                ));
                label_line.push(' ');
            }
            BlockType::Stadium => {
                label_line.push_str(&format!(
                    "({}{}{}) ",
                    " ".repeat(pad_left),
                    label,
                    " ".repeat(pad_right)
                ));
            }
            _ => {
                label_line.push_str(&format!(
                    "│{}{}{}│ ",
                    " ".repeat(pad_left),
                    label,
                    " ".repeat(pad_right)
                ));
            }
        }
    }
    lines.push(label_line.trim_end().to_string());

    // Bottom border
    let mut bottom = String::from(indent);
    for id in chunk {
        let block = blocks_map.get(*id);
        let bt = block.map(|b| &b.block_type).unwrap_or(&BlockType::Square);
        match bt {
            BlockType::Space => {
                bottom.push_str(&" ".repeat(cell_width + 2));
                bottom.push(' ');
            }
            BlockType::Round => {
                bottom.push_str(&format!("╰{}╯ ", "─".repeat(cell_width)));
            }
            BlockType::Diamond => {
                let total_pad = cell_width + 1;
                let left = total_pad / 2;
                let right = total_pad - left;
                bottom.push_str(&format!("{}◇{} ", " ".repeat(left), " ".repeat(right)));
            }
            BlockType::Stadium => {
                bottom.push_str(&format!("╰{}╯ ", "─".repeat(cell_width)));
            }
            _ => {
                bottom.push_str(&format!("└{}┘ ", "─".repeat(cell_width)));
            }
        }
    }
    lines.push(bottom.trim_end().to_string());
}

/// Render a row that contains at least one composite block.
/// Composites are drawn as bordered containers with their children inside.
fn render_mixed_row(
    db: &BlockDb,
    chunk: &[&str],
    cell_width: usize,
    indent: &str,
    lines: &mut Vec<String>,
) {
    let blocks_map = db.get_blocks();

    // Pre-render each cell as a set of lines so we can align heights
    let mut cell_renders: Vec<Vec<String>> = Vec::new();
    let mut cell_widths: Vec<usize> = Vec::new();

    for id in chunk {
        let block = blocks_map.get(*id);
        let bt = block.map(|b| &b.block_type).unwrap_or(&BlockType::Square);

        if *bt == BlockType::Composite {
            let children: Vec<&str> = db.get_children(id).into_iter().collect();
            let child_columns = block.and_then(|b| b.columns).unwrap_or(2);
            let child_cell_width = calc_cell_width(db, &children);

            // Calculate composite inner width
            let cols_in_use = child_columns.min(children.len());
            let inner_width = if cols_in_use > 0 {
                cols_in_use * (child_cell_width + 3) - 1
            } else {
                cell_width
            };
            let composite_width = inner_width + 2; // for outer border padding

            let mut cell_lines = Vec::new();

            // Top border of composite
            cell_lines.push(format!("┌{}┐", "─".repeat(composite_width)));

            // Render children inside
            let mut inner_lines: Vec<String> = Vec::new();
            render_block_rows(
                db,
                &children,
                child_columns,
                child_cell_width,
                "  ",
                &mut inner_lines,
            );

            // Remove trailing empty lines
            while inner_lines.last().is_some_and(|l| l.trim().is_empty()) {
                inner_lines.pop();
            }

            for il in &inner_lines {
                let content_len = il.chars().count();
                let pad = composite_width.saturating_sub(content_len);
                cell_lines.push(format!("│{}{}│", il, " ".repeat(pad)));
            }

            // Bottom border of composite
            cell_lines.push(format!("└{}┘", "─".repeat(composite_width)));

            cell_widths.push(composite_width + 2); // +2 for the │ chars
            cell_renders.push(cell_lines);
        } else if *bt == BlockType::Space {
            cell_widths.push(cell_width + 2);
            cell_renders.push(vec![
                " ".repeat(cell_width + 2),
                " ".repeat(cell_width + 2),
                " ".repeat(cell_width + 2),
            ]);
        } else {
            // Simple block rendered as 3 lines
            let label = block
                .and_then(|b| b.label.as_ref())
                .map(|l| l.as_str())
                .unwrap_or(id);
            let pad_total = cell_width.saturating_sub(label.chars().count());
            let pad_left = pad_total / 2;
            let pad_right = pad_total - pad_left;

            let (top_ch, side_l, side_r, bot_ch) = shape_chars(bt);
            cell_widths.push(cell_width + 2);
            cell_renders.push(vec![
                format!("{}{}{}", top_ch.0, "─".repeat(cell_width), top_ch.1),
                format!(
                    "{}{}{}{}{}",
                    side_l,
                    " ".repeat(pad_left),
                    label,
                    " ".repeat(pad_right),
                    side_r
                ),
                format!("{}{}{}", bot_ch.0, "─".repeat(cell_width), bot_ch.1),
            ]);
        }
    }

    // Find max height across cells in this row
    let max_height = cell_renders.iter().map(|c| c.len()).max().unwrap_or(3);

    // Emit lines, padding shorter cells vertically
    for row_idx in 0..max_height {
        let mut line = String::from(indent);
        for (cell_idx, cell) in cell_renders.iter().enumerate() {
            if row_idx < cell.len() {
                line.push_str(&cell[row_idx]);
            } else {
                // Pad with spaces to match width
                line.push_str(&" ".repeat(cell_widths[cell_idx]));
            }
            line.push(' ');
        }
        lines.push(line.trim_end().to_string());
    }
}

/// Return shape-specific border characters: ((top_left, top_right), side_left, side_right, (bot_left, bot_right))
fn shape_chars(
    bt: &BlockType,
) -> (
    (&'static str, &'static str),
    &'static str,
    &'static str,
    (&'static str, &'static str),
) {
    match bt {
        BlockType::Round => (("╭", "╮"), "│", "│", ("╰", "╯")),
        BlockType::Diamond => ((" ", " "), "<", ">", (" ", " ")),
        BlockType::Stadium => (("╭", "╮"), "(", ")", ("╰", "╯")),
        _ => (("┌", "┐"), "│", "│", ("└", "┘")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_block() {
        let db = BlockDb::new();
        let output = render_block_ascii(&db).unwrap();
        assert!(output.contains("empty block"));
    }

    #[test]
    fn gallery_block_renders() {
        let input = std::fs::read_to_string("docs/sources/block.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Block(db) => db,
            _ => panic!("Expected block"),
        };
        let output = render_block_ascii(&db).unwrap();
        assert!(!output.trim().is_empty(), "Output should not be empty");
        // Should have box-drawing characters
        assert!(
            output.contains('┌'),
            "Should have box corners\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn has_box_structure() {
        let input = std::fs::read_to_string("docs/sources/block.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Block(db) => db,
            _ => panic!("Expected block"),
        };
        let output = render_block_ascii(&db).unwrap();
        assert!(output.contains('┌'), "Output:\n{}", output);
        assert!(output.contains('┘'), "Output:\n{}", output);
        assert!(output.contains('│'), "Output:\n{}", output);
    }

    #[test]
    fn composite_children_visible() {
        let input = r#"block-beta
  columns 3
  A["Parent"]
  block:container
    columns 2
    D["Nested 1"]
    E["Nested 2"]
  end
  F["Sibling"]"#;
        let diagram = crate::parse(input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Block(db) => db,
            _ => panic!("Expected block"),
        };
        let output = render_block_ascii(&db).unwrap();
        assert!(
            output.contains("Nested 1"),
            "Nested child 1 should be visible\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Nested 2"),
            "Nested child 2 should be visible\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn diamond_shape_distinct() {
        let input = "block-beta\n  A{\"Diamond\"}";
        let diagram = crate::parse(input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Block(db) => db,
            _ => panic!("Expected block"),
        };
        let output = render_block_ascii(&db).unwrap();
        assert!(
            output.contains('◇') || output.contains('/') || output.contains('<'),
            "Diamond shape should use distinct characters\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn stadium_shape_distinct() {
        let input = "block-beta\n  A([\"Stadium\"])";
        let diagram = crate::parse(input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Block(db) => db,
            _ => panic!("Expected block"),
        };
        let output = render_block_ascii(&db).unwrap();
        assert!(
            output.contains('(') || output.contains(')'),
            "Stadium shape should use rounded ends\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn round_shape_distinct() {
        let input = "block-beta\n  A(\"Rounded\")";
        let diagram = crate::parse(input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Block(db) => db,
            _ => panic!("Expected block"),
        };
        let output = render_block_ascii(&db).unwrap();
        assert!(
            output.contains('╭') || output.contains('╰'),
            "Rounded shape should use rounded corners\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn space_blocks_invisible() {
        let input = "block-beta\n  columns 3\n  A[\"Left\"]\n  space\n  B[\"Right\"]";
        let diagram = crate::parse(input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Block(db) => db,
            _ => panic!("Expected block"),
        };
        let output = render_block_ascii(&db).unwrap();
        assert!(
            !output.contains("space"),
            "Space blocks should not show a label\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn block_complex_gallery_renders() {
        let input = std::fs::read_to_string("docs/sources/block_complex.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Block(db) => db,
            _ => panic!("Expected block"),
        };
        let output = render_block_ascii(&db).unwrap();

        // Must show all named blocks
        assert!(
            output.contains("Square Block"),
            "Missing 'Square Block'\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Rounded Block"),
            "Missing 'Rounded Block'\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Diamond"),
            "Missing 'Diamond'\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Nested 1"),
            "Missing 'Nested 1'\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Nested 2"),
            "Missing 'Nested 2'\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Stadium"),
            "Missing 'Stadium'\nOutput:\n{}",
            output
        );

        // Should not render space as visible block
        assert!(
            !output.contains("space_"),
            "Space rendered as visible block\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn diamond_and_square_same_row_width() {
        // With 3 columns and a diamond in the middle, all three lines of the row
        // must have equal character widths. If the diamond cell is narrower,
        // the top/bottom lines will be shorter than the label line.
        let input = "block-beta\n  columns 3\n  A[\"Left\"]\n  B{\"Mid\"}\n  C[\"Right\"]";
        let diagram = crate::parse(input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Block(db) => db,
            _ => panic!("Expected block"),
        };
        let output = render_block_ascii(&db).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert!(
            lines.len() >= 3,
            "Need at least 3 lines\nOutput:\n{}",
            output
        );

        let top_chars = lines[0].chars().count();
        let label_chars = lines[1].chars().count();
        let bottom_chars = lines[2].chars().count();
        assert_eq!(
            top_chars, label_chars,
            "Top border ({}) and label ({}) char widths must match\nOutput:\n{}",
            top_chars, label_chars, output
        );
        assert_eq!(
            label_chars, bottom_chars,
            "Label ({}) and bottom border ({}) char widths must match\nOutput:\n{}",
            label_chars, bottom_chars, output
        );
    }
}
