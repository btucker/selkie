//! ASCII renderer for block diagrams.
//!
//! Renders blocks in a grid layout with edges shown as flow connections.
//! Uses box-drawing characters for block borders and arrows for connections.

use crate::diagrams::block::BlockDb;
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

    // Render blocks in rows
    let block_order = db.get_block_order();

    // Filter to top-level blocks (not children of composites)
    let top_level: Vec<&str> = block_order
        .iter()
        .filter(|id| {
            db.get_blocks()
                .get(id.as_str())
                .is_some_and(|b| b.parent_id.is_none())
        })
        .map(|s| s.as_str())
        .collect();

    // Calculate cell width from longest label
    let max_label_len = top_level
        .iter()
        .filter_map(|id| db.get_blocks().get(*id))
        .map(|b| {
            b.label
                .as_ref()
                .map(|l| l.chars().count())
                .unwrap_or(b.id.chars().count())
        })
        .max()
        .unwrap_or(8);
    let cell_width = max_label_len + 4; // padding + borders

    for chunk in top_level.chunks(columns) {
        // Top border
        let mut top_line = String::from("  ");
        for _ in chunk {
            top_line.push_str(&format!("┌{}┐ ", "─".repeat(cell_width)));
        }
        lines.push(top_line.trim_end().to_string());

        // Label row
        let mut label_line = String::from("  ");
        for id in chunk {
            let block = db.get_blocks().get(*id);
            let label = block
                .and_then(|b| b.label.as_ref())
                .map(|l| l.as_str())
                .unwrap_or(id);

            let pad_total = cell_width.saturating_sub(label.chars().count());
            let pad_left = pad_total / 2;
            let pad_right = pad_total - pad_left;
            label_line.push_str(&format!(
                "│{}{}{}│ ",
                " ".repeat(pad_left),
                label,
                " ".repeat(pad_right)
            ));
        }
        lines.push(label_line.trim_end().to_string());

        // Bottom border
        let mut bottom_line = String::from("  ");
        for _ in chunk {
            bottom_line.push_str(&format!("└{}┘ ", "─".repeat(cell_width)));
        }
        lines.push(bottom_line.trim_end().to_string());

        // Inter-row spacing with potential edge arrows
        lines.push(String::new());
    }

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
}
