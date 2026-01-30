//! ASCII renderer for treemap diagrams.
//!
//! Since proportional area layout is complex in character art, treemaps
//! are rendered as an indented tree with value bars showing relative sizes.

use crate::diagrams::treemap::TreemapDb;
use crate::error::Result;

const BAR_WIDTH: usize = 20;
const FULL_BLOCK: char = '█';
const HALF_BLOCK: char = '▌';

/// Render a treemap as character art.
pub fn render_treemap_ascii(db: &TreemapDb) -> Result<String> {
    let root_nodes = db.get_root_nodes();
    if root_nodes.is_empty() {
        let title = db.get_title();
        if !title.is_empty() {
            return Ok(format!("{}\n\n(empty treemap)\n", title));
        }
        return Ok("(empty treemap)\n".to_string());
    }

    let mut lines: Vec<String> = Vec::new();

    // Title
    let title = db.get_title();
    if !title.is_empty() {
        lines.push(title.to_string());
        lines.push("─".repeat(title.chars().count().max(40)));
    }

    // Calculate total value for scaling
    let total_value = calculate_total(root_nodes);
    let max_value = find_max_leaf(root_nodes);

    // Render tree
    for node in root_nodes {
        render_node(&mut lines, node, 0, max_value);
    }

    // Total
    if total_value > 0.0 {
        lines.push(String::new());
        let total_str = if total_value.fract() == 0.0 {
            format!("{}", total_value as i64)
        } else {
            format!("{:.1}", total_value)
        };
        lines.push(format!("  Total: {}", total_str));
    }

    lines.push(String::new());
    Ok(lines.join("\n"))
}

fn render_node(
    lines: &mut Vec<String>,
    node: &crate::diagrams::treemap::TreemapNode,
    depth: usize,
    max_value: f64,
) {
    let indent = "  ".repeat(depth + 1);

    if node.is_leaf() {
        let value = node.value.unwrap_or(0.0);
        let value_str = if value.fract() == 0.0 {
            format!("{}", value as i64)
        } else {
            format!("{:.1}", value)
        };

        // Bar
        let pct = if max_value > 0.0 {
            value / max_value
        } else {
            0.0
        };
        let bar_cells = (pct * BAR_WIDTH as f64).round() as usize;
        let mut bar = String::new();
        for _ in 0..bar_cells {
            bar.push(FULL_BLOCK);
        }
        if bar.is_empty() && value > 0.0 {
            bar.push(HALF_BLOCK);
        }

        lines.push(format!("{}├─ {} │{} {}", indent, node.name, bar, value_str));
    } else {
        // Section header
        lines.push(format!("{}┌─ {} ─┐", indent, node.name));
        for child in &node.children {
            render_node(lines, child, depth + 1, max_value);
        }
        lines.push(format!("{}└────┘", indent));
    }
}

fn calculate_total(nodes: &[crate::diagrams::treemap::TreemapNode]) -> f64 {
    let mut total = 0.0;
    for node in nodes {
        if let Some(value) = node.value {
            total += value;
        }
        total += calculate_total(&node.children);
    }
    total
}

fn find_max_leaf(nodes: &[crate::diagrams::treemap::TreemapNode]) -> f64 {
    let mut max = 0.0f64;
    for node in nodes {
        if let Some(value) = node.value {
            max = max.max(value);
        }
        max = max.max(find_max_leaf(&node.children));
    }
    max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_treemap() {
        let db = TreemapDb::new();
        let output = render_treemap_ascii(&db).unwrap();
        assert!(output.contains("empty treemap"));
    }

    #[test]
    fn gallery_treemap_renders() {
        let input = std::fs::read_to_string("docs/sources/treemap.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Treemap(db) => db,
            _ => panic!("Expected treemap"),
        };
        let output = render_treemap_ascii(&db).unwrap();
        assert!(output.contains("Category A"), "Output:\n{}", output);
        assert!(output.contains("Category B"), "Output:\n{}", output);
        assert!(output.contains("Item A1"), "Output:\n{}", output);
    }

    #[test]
    fn values_appear() {
        let input = std::fs::read_to_string("docs/sources/treemap.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Treemap(db) => db,
            _ => panic!("Expected treemap"),
        };
        let output = render_treemap_ascii(&db).unwrap();
        // Values from the sample: 10, 20, 15, 25
        assert!(output.contains("10"), "Output:\n{}", output);
        assert!(output.contains("25"), "Output:\n{}", output);
    }

    #[test]
    fn has_tree_structure() {
        let input = std::fs::read_to_string("docs/sources/treemap.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Treemap(db) => db,
            _ => panic!("Expected treemap"),
        };
        let output = render_treemap_ascii(&db).unwrap();
        assert!(output.contains('├'), "Output:\n{}", output);
        assert!(
            output.contains(FULL_BLOCK),
            "Should have value bars\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn total_appears() {
        let input = std::fs::read_to_string("docs/sources/treemap.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Treemap(db) => db,
            _ => panic!("Expected treemap"),
        };
        let output = render_treemap_ascii(&db).unwrap();
        assert!(output.contains("Total: 70"), "Output:\n{}", output);
    }
}
