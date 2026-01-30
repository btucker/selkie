//! ASCII renderer for kanban board diagrams.
//!
//! Renders kanban columns with their cards in a vertical layout,
//! using box-drawing characters for column headers and card borders.

use crate::diagrams::kanban::KanbanDb;
use crate::error::Result;

/// Render a kanban board as character art.
pub fn render_kanban_ascii(db: &KanbanDb) -> Result<String> {
    let sections = db.get_sections();
    if sections.is_empty() {
        return Ok("(empty kanban board)\n".to_string());
    }

    let mut lines: Vec<String> = Vec::new();

    for section in &sections {
        let children = db.get_children(&section.id);

        // Column header
        let header = &section.label;
        let header_width = header.chars().count().max(20);
        lines.push(format!("┌{}┐", "─".repeat(header_width + 2)));
        let pad_total = header_width.saturating_sub(header.chars().count());
        let pad_left = pad_total / 2;
        let pad_right = pad_total - pad_left;
        lines.push(format!(
            "│ {}{}{} │",
            " ".repeat(pad_left),
            header,
            " ".repeat(pad_right)
        ));
        lines.push(format!("├{}┤", "─".repeat(header_width + 2)));

        // Cards
        if children.is_empty() {
            lines.push(format!("│ {:width$} │", "(empty)", width = header_width));
        } else {
            for child in &children {
                let label = &child.label;
                let card_text = if label.chars().count() > header_width {
                    let truncated: String = label.chars().take(header_width - 1).collect();
                    format!("{}…", truncated)
                } else {
                    format!("{:width$}", label, width = header_width)
                };
                lines.push(format!("│ {} │", card_text));
            }
        }

        lines.push(format!("└{}┘", "─".repeat(header_width + 2)));
        lines.push(String::new());
    }

    Ok(lines.join("\n"))
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
